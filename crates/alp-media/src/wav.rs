//! WAV encoding with selectable bit depth.
//!
//! Writes are atomic: a temporary sibling is finalized and renamed over the
//! destination. Encoding is an offline/worker operation, never real-time.

use std::path::Path;
use std::str::FromStr;

use alp_core::{commit_temp, create_temp_sibling};

use crate::error::MediaError;
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
    let path = path.as_ref();
    let (temp, file) = create_temp_sibling(path)?;
    let result = write_to(file, buffer, bit_depth);
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
            for sample in buffer.samples() {
                writer
                    .write_sample(float_to_i16(*sample))
                    .map_err(|error| MediaError::Encode(error.to_string()))?;
            }
        }
        WavBitDepth::Int24 => {
            for sample in buffer.samples() {
                writer
                    .write_sample(float_to_i24(*sample))
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

fn float_to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16
}

fn float_to_i24(sample: f32) -> i32 {
    (sample.clamp(-1.0, 1.0) * 8_388_607.0).round() as i32
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
