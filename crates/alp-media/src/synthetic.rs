//! Deterministic synthetic audio for tests and demos.
//!
//! The generator streams straight to disk, so a one-hour fixture does not need
//! one hour of RAM. Used by `alp-cli gen` and by media tests.

use std::path::Path;

use alp_core::{commit_temp, create_temp_sibling};

use crate::error::MediaError;

/// Write a deterministic tone/noise fixture as 16-bit PCM WAV.
pub fn write_test_tone(
    path: impl AsRef<Path>,
    seconds: f64,
    sample_rate: u32,
    channels: u16,
    frequency: f32,
    amplitude: f32,
) -> Result<(), MediaError> {
    if !seconds.is_finite() || seconds <= 0.0 {
        return Err(MediaError::InvalidBuffer("seconds must be positive".into()));
    }
    if channels == 0 || channels > 32 {
        return Err(MediaError::InvalidBuffer(
            "unsupported channel count".into(),
        ));
    }
    if !frequency.is_finite() || frequency <= 0.0 {
        return Err(MediaError::InvalidBuffer(
            "frequency must be positive".into(),
        ));
    }
    if !amplitude.is_finite() || !(0.0..=1.0).contains(&amplitude) {
        return Err(MediaError::InvalidBuffer(
            "amplitude must be within 0..=1".into(),
        ));
    }

    let path = path.as_ref();
    let (temp, file) = create_temp_sibling(path)?;
    let result = write_to(file, seconds, sample_rate, channels, frequency, amplitude);
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
    seconds: f64,
    sample_rate: u32,
    channels: u16,
    frequency: f32,
    amplitude: f32,
) -> Result<(), MediaError> {
    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let frames = (seconds * f64::from(sample_rate)).round();
    if !frames.is_finite() || frames > u64::MAX as f64 {
        return Err(MediaError::InvalidBuffer("duration is too large".into()));
    }
    let frames = frames as u64;
    let mut writer =
        hound::WavWriter::new(file, spec).map_err(|error| MediaError::Encode(error.to_string()))?;
    let mut noise = 0x1234_5678u64;
    for frame in 0..frames {
        let time = frame as f32 / sample_rate as f32;
        let carrier = (std::f32::consts::TAU * frequency * time).sin() * 0.7
            + (std::f32::consts::TAU * frequency * 2.0 * time).sin() * 0.3;
        let envelope = 0.55 + 0.45 * (std::f32::consts::TAU * 0.5 * time).sin().abs();
        noise = noise
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let hiss = ((noise >> 40) as f32 / 8_388_608.0 - 0.5) * 0.02;
        let value = (carrier * envelope + hiss) * amplitude;
        let sample = (value.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16;
        for _ in 0..channels {
            writer
                .write_sample(sample)
                .map_err(|error| MediaError::Encode(error.to_string()))?;
        }
    }
    writer
        .finalize()
        .map_err(|error| MediaError::Encode(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode;

    #[test]
    fn generated_tone_decodes_with_expected_shape() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("tone.wav");
        write_test_tone(&path, 0.3, 48_000, 2, 440.0, 0.5).expect("write");
        let info = decode::probe(&path).expect("probe");
        assert_eq!(info.sample_rate, 48_000);
        assert_eq!(info.channels, 2);
        assert_eq!(info.frames, Some(14_400));
        let buffer = decode::decode(&path).expect("decode");
        assert!(buffer.peak() > 0.2);
    }

    #[test]
    fn invalid_arguments_are_rejected() {
        assert!(write_test_tone("/tmp/nope.wav", 0.0, 48_000, 1, 440.0, 0.5).is_err());
        assert!(write_test_tone("/tmp/nope.wav", 1.0, 48_000, 0, 440.0, 0.5).is_err());
        assert!(write_test_tone("/tmp/nope.wav", 1.0, 48_000, 1, 440.0, 2.0).is_err());
    }
}
