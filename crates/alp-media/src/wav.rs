//! WAV encoding with selectable bit depth.
//!
//! Writes are atomic: a temporary sibling is finalized and renamed over the
//! destination. Encoding is an offline/worker operation, never real-time.

use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::str::FromStr;

use alp_core::{commit_temp, create_temp_sibling};

use crate::dither::{DitherMode, Quantizer};
use crate::error::MediaError;
use crate::export::AudioMetadata;
use crate::pcm::PcmBuffer;

/// Output sample format for a WAV file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WavBitDepth {
    /// 16-bit signed integer PCM.
    Int16,
    /// 24-bit signed integer PCM stored in 32-bit words.
    Int24,
    /// 32-bit IEEE float PCM.
    Float32,
}

impl WavBitDepth {
    /// Bits per sample written to the WAV header.
    #[must_use]
    pub fn bits(self) -> u16 {
        match self {
            Self::Int16 => 16,
            Self::Int24 => 24,
            Self::Float32 => 32,
        }
    }

    /// Short label used by the CLI and UI.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Int16 => "int16",
            Self::Int24 => "int24",
            Self::Float32 => "float32",
        }
    }
}

impl FromStr for WavBitDepth {
    type Err = MediaError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "16" | "i16" | "int16" => Ok(Self::Int16),
            "24" | "i24" | "int24" => Ok(Self::Int24),
            "32" | "f32" | "float" | "float32" => Ok(Self::Float32),
            other => Err(MediaError::Encode(format!(
                "unknown bit depth {other:?} (use 16, 24 or float32)"
            ))),
        }
    }
}

/// Write a PCM buffer to a WAV file atomically.
pub fn write_wav(
    path: impl AsRef<Path>,
    buffer: &PcmBuffer,
    bit_depth: WavBitDepth,
) -> Result<(), MediaError> {
    write_wav_with(
        path,
        buffer,
        bit_depth,
        DitherMode::None,
        &AudioMetadata::default(),
    )
}

/// Write a WAV file with dither and RIFF INFO metadata.
pub(crate) fn write_wav_with(
    path: impl AsRef<Path>,
    buffer: &PcmBuffer,
    bit_depth: WavBitDepth,
    dither: DitherMode,
    metadata: &AudioMetadata,
) -> Result<(), MediaError> {
    let path = path.as_ref();
    let (temp, file) = create_temp_sibling(path)?;
    let result =
        write_to(file, buffer, bit_depth, dither).and_then(|()| append_riff_info(&temp, metadata));
    match result {
        Ok(()) => {
            commit_temp(&temp, path)?;
            Ok(())
        }
        Err(error) => {
            let _ = std::fs::remove_file(&temp);
            Err(error)
        }
    }
}

fn write_to(
    file: std::fs::File,
    buffer: &PcmBuffer,
    bit_depth: WavBitDepth,
    dither: DitherMode,
) -> Result<(), MediaError> {
    let spec = hound::WavSpec {
        channels: buffer.channels(),
        sample_rate: buffer.sample_rate(),
        bits_per_sample: bit_depth.bits(),
        sample_format: match bit_depth {
            WavBitDepth::Float32 => hound::SampleFormat::Float,
            WavBitDepth::Int16 | WavBitDepth::Int24 => hound::SampleFormat::Int,
        },
    };
    let mut writer =
        hound::WavWriter::new(file, spec).map_err(|error| MediaError::Encode(error.to_string()))?;
    match bit_depth {
        WavBitDepth::Int16 => {
            let mut quantizer = Quantizer::new(i32::from(i16::MAX), dither);
            for sample in buffer.samples() {
                writer
                    .write_sample(quantizer.quantize(*sample) as i16)
                    .map_err(|error| MediaError::Encode(error.to_string()))?;
            }
        }
        WavBitDepth::Int24 => {
            let mut quantizer = Quantizer::new(8_388_607, dither);
            for sample in buffer.samples() {
                writer
                    .write_sample(quantizer.quantize(*sample))
                    .map_err(|error| MediaError::Encode(error.to_string()))?;
            }
        }
        WavBitDepth::Float32 => {
            for sample in buffer.samples() {
                writer
                    .write_sample(*sample)
                    .map_err(|error| MediaError::Encode(error.to_string()))?;
            }
        }
    }
    writer
        .finalize()
        .map_err(|error| MediaError::Encode(error.to_string()))
}

fn append_riff_info(path: &Path, metadata: &AudioMetadata) -> Result<(), MediaError> {
    let tags = metadata.riff_tags();
    if tags.is_empty() {
        return Ok(());
    }
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(MediaError::Io)?;
    let mut header = [0u8; 12];
    file.read_exact(&mut header).map_err(MediaError::Io)?;
    if &header[0..4] != b"RIFF" || &header[8..12] != b"WAVE" {
        return Err(MediaError::Encode("not a RIFF/WAVE file".into()));
    }

    let mut body = Vec::from(*b"INFO");
    for (id, value) in tags {
        let mut data = value.as_bytes().to_vec();
        data.push(0);
        if data.len().is_multiple_of(2) {
            // Already even, including the NUL terminator.
        } else {
            data.push(0);
        }
        body.extend_from_slice(id);
        body.extend_from_slice(
            &u32::try_from(data.len())
                .map_err(|_| MediaError::Encode("RIFF tag is too long".into()))?
                .to_le_bytes(),
        );
        body.extend_from_slice(&data);
    }

    let end = file.seek(SeekFrom::End(0)).map_err(MediaError::Io)?;
    file.write_all(b"LIST").map_err(MediaError::Io)?;
    file.write_all(
        &u32::try_from(body.len())
            .map_err(|_| MediaError::Encode("RIFF INFO chunk is too long".into()))?
            .to_le_bytes(),
    )
    .map_err(MediaError::Io)?;
    file.write_all(&body).map_err(MediaError::Io)?;
    let new_len = end + 8 + body.len() as u64;
    file.seek(SeekFrom::Start(4)).map_err(MediaError::Io)?;
    file.write_all(
        &u32::try_from(new_len - 8)
            .map_err(|_| MediaError::Encode("WAV file is too long".into()))?
            .to_le_bytes(),
    )
    .map_err(MediaError::Io)?;
    file.sync_all().map_err(MediaError::Io)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_16_and_float_wav_headers() {
        let dir = tempfile::tempdir().expect("tempdir");
        let buffer = PcmBuffer::new(48_000, 2, vec![0.0, 0.5, -0.5, 0.25]).expect("buffer");
        for depth in [WavBitDepth::Int16, WavBitDepth::Float32] {
            let path = dir.path().join(format!("{}.wav", depth.label()));
            write_wav(&path, &buffer, depth).expect("write");
            let reader = hound::WavReader::open(&path).expect("read");
            assert_eq!(reader.spec().sample_rate, 48_000);
            assert_eq!(reader.spec().channels, 2);
            assert_eq!(reader.spec().bits_per_sample, depth.bits());
        }
    }

    #[test]
    fn bit_depth_parses() {
        assert_eq!(
            "16".parse::<WavBitDepth>().expect("parse"),
            WavBitDepth::Int16
        );
        assert_eq!(
            "float32".parse::<WavBitDepth>().expect("parse"),
            WavBitDepth::Float32
        );
        assert!("8".parse::<WavBitDepth>().is_err());
    }
}
