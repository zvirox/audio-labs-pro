//! Symphonia-backed decoding into interleaved f32.
//!
//! `probe` reads container/codec parameters only; `decode_path` streams decoded
//! packets into a caller callback and counts frames. Whole-file decoding is a
//! convenience wrapper and is intended for short clips and offline work.

use std::path::Path;

use symphonia::core::codecs::audio::well_known::{
    CODEC_ID_AAC, CODEC_ID_FLAC, CODEC_ID_MP3, CODEC_ID_OPUS, CODEC_ID_PCM_F32LE,
    CODEC_ID_PCM_S16LE, CODEC_ID_PCM_S24LE, CODEC_ID_PCM_S32LE, CODEC_ID_VORBIS,
};
use symphonia::core::codecs::audio::{AudioCodecId, AudioDecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

use crate::error::MediaError;
use crate::pcm::PcmBuffer;

/// Container-level information about an audio file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaInfo {
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Channel count.
    pub channels: u16,
    /// Frame count when the container declares it.
    pub frames: Option<u64>,
    /// Debug name of the codec.
    pub codec: String,
}

/// Decode an entire file into memory.
pub fn decode(path: impl AsRef<Path>) -> Result<PcmBuffer, MediaError> {
    let mut samples = Vec::new();
    let mut channels = 0;
    let info = decode_path(path, |chunk, chunk_channels| {
        channels = chunk_channels;
        samples.extend_from_slice(chunk);
        Ok(())
    })?;
    PcmBuffer::new(info.sample_rate, channels, samples)
}

/// Probe a file without decoding audio.
pub fn probe(path: impl AsRef<Path>) -> Result<MediaInfo, MediaError> {
    let format = open_format(path.as_ref())?;
    let track = format
        .default_track(TrackType::Audio)
        .ok_or_else(|| MediaError::Decode("no audio track".into()))?;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|params| params.audio())
        .ok_or_else(|| MediaError::Decode("missing audio codec parameters".into()))?;
    Ok(MediaInfo {
        sample_rate: params.sample_rate.unwrap_or(48_000),
        channels: params
            .channels
            .as_ref()
            .map_or(1, |channels| channels.count() as u16),
        frames: track.num_frames,
        codec: codec_label(params.codec),
    })
}

/// Stream decoded interleaved f32 chunks to `on_chunk`.
///
/// The callback receives the sample slice and the channel count. Chunk
/// boundaries are codec packets, not a fixed frame size.
pub fn decode_path<F>(path: impl AsRef<Path>, mut on_chunk: F) -> Result<MediaInfo, MediaError>
where
    F: FnMut(&[f32], u16) -> Result<(), MediaError>,
{
    let path = path.as_ref();
    let mut format = open_format(path)?;
    let track = format
        .default_track(TrackType::Audio)
        .ok_or_else(|| MediaError::Decode("no audio track".into()))?;
    let track_id = track.id;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|params| params.audio())
        .ok_or_else(|| MediaError::Decode("missing audio codec parameters".into()))?
        .clone();
    let codec = codec_label(params.codec);
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&params, &AudioDecoderOptions::default())
        .map_err(|error| MediaError::Decode(format!("could not create decoder: {error}")))?;

    let mut scratch: Vec<f32> = Vec::new();
    let mut frames = 0u64;
    let mut sample_rate = params.sample_rate.unwrap_or(48_000);
    let mut channels = params
        .channels
        .as_ref()
        .map_or(1, |channels| channels.count() as u16);
    let mut decoded_packets = 0u64;

    loop {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            Err(SymphoniaError::ResetRequired) => break,
            Err(error) => return Err(MediaError::Decode(format!("next_packet: {error}"))),
        };
        if packet.track_id != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(SymphoniaError::IoError(_)) | Err(SymphoniaError::DecodeError(_)) => continue,
            Err(error) => return Err(MediaError::Decode(format!("decoder: {error}"))),
        };

        let spec = decoded.spec();
        let chunk_rate = spec.rate();
        let chunk_channels = spec.channels().count() as u16;
        if decoded_packets == 0 {
            sample_rate = chunk_rate;
            channels = chunk_channels;
        } else if chunk_rate != sample_rate || chunk_channels != channels {
            return Err(MediaError::Unsupported(
                "stream changes sample rate or channel count mid-file".into(),
            ));
        }

        let sample_count = decoded.frames() * usize::from(chunk_channels);
        scratch.clear();
        scratch.reserve(sample_count);
        decoded.copy_to_vec_interleaved(&mut scratch);
        frames += decoded.frames() as u64;
        decoded_packets += 1;
        on_chunk(&scratch, channels)?;
    }

    if decoded_packets == 0 {
        return Err(MediaError::Decode("no audio frames decoded".into()));
    }

    Ok(MediaInfo {
        sample_rate,
        channels,
        frames: Some(frames),
        codec,
    })
}

fn codec_label(codec: AudioCodecId) -> String {
    let label = match codec {
        CODEC_ID_FLAC => "flac",
        CODEC_ID_VORBIS => "vorbis",
        CODEC_ID_OPUS => "opus",
        CODEC_ID_MP3 => "mp3",
        CODEC_ID_AAC => "aac",
        CODEC_ID_PCM_S16LE | CODEC_ID_PCM_S24LE | CODEC_ID_PCM_S32LE | CODEC_ID_PCM_F32LE => "pcm",
        _ => return format!("{codec:?}"),
    };
    label.to_owned()
}

fn open_format(path: &Path) -> Result<Box<dyn FormatReader>, MediaError> {
    let file = std::fs::File::open(path).map_err(|error| {
        MediaError::Io(std::io::Error::new(
            error.kind(),
            format!("open {}: {error}", path.display()),
        ))
    })?;
    let stream = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(extension) = path.extension().and_then(|extension| extension.to_str()) {
        hint.with_extension(extension);
    }
    symphonia::default::get_probe()
        .probe(
            &hint,
            stream,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|error| MediaError::Decode(format!("probe: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synthetic::write_test_tone;

    #[test]
    fn decodes_a_generated_wav() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("tone.wav");
        write_test_tone(&path, 0.25, 48_000, 2, 440.0, 0.25).expect("generate");

        let info = probe(&path).expect("probe");
        assert_eq!(info.sample_rate, 48_000);
        assert_eq!(info.channels, 2);

        let buffer = decode(&path).expect("decode");
        assert_eq!(buffer.sample_rate(), 48_000);
        assert_eq!(buffer.channels(), 2);
        assert!((buffer.duration_secs() - 0.25).abs() < 0.01);
        assert!(buffer.peak() > 0.1);
    }
}
