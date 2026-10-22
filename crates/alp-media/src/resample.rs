//! Offline sample-rate conversion using rubato's FFT resampler.
//!
//! Resampling is never performed in the audio callback. Import and export
//! resample in worker threads and store the result as a [`PcmBuffer`].

use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Resampler};

use crate::error::MediaError;
use crate::pcm::PcmBuffer;

/// Resample a buffer to `target_rate`, preserving channel count and duration.
pub fn resample(buffer: &PcmBuffer, target_rate: u32) -> Result<PcmBuffer, MediaError> {
    if target_rate == buffer.sample_rate() {
        return Ok(buffer.clone());
    }
    if buffer.frames() == 0 {
        return PcmBuffer::silence(target_rate, buffer.channels(), 0);
    }

    let channels = usize::from(buffer.channels());
    let frames = usize::try_from(buffer.frames())
        .map_err(|_| MediaError::Resample("frame count too large".into()))?;
    let mut resampler = Fft::<f32>::new(
        buffer.sample_rate() as usize,
        target_rate as usize,
        1024,
        channels,
        FixedSync::Both,
    )
    .map_err(|error| MediaError::Resample(error.to_string()))?;

    let input = InterleavedSlice::new(buffer.samples(), channels, frames)
        .map_err(|error| MediaError::Resample(error.to_string()))?;
    let output = resampler
        .process_all(&input, frames, None)
        .map_err(|error| MediaError::Resample(error.to_string()))?;
    PcmBuffer::new(target_rate, buffer.channels(), output.take_data())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(sample_rate: u32, channels: u16, seconds: f64, frequency: f32) -> PcmBuffer {
        let frames = (seconds * f64::from(sample_rate)).round() as usize;
        let mut samples = Vec::with_capacity(frames * usize::from(channels));
        for frame in 0..frames {
            let value =
                (std::f32::consts::TAU * frequency * frame as f32 / sample_rate as f32).sin() * 0.5;
            for _ in 0..channels {
                samples.push(value);
            }
        }
        PcmBuffer::new(sample_rate, channels, samples).expect("buffer")
    }

    #[test]
    fn identity_returns_a_copy() {
        let buffer = sine(48_000, 1, 0.1, 440.0);
        let output = resample(&buffer, 48_000).expect("resample");
        assert_eq!(buffer, output);
    }

    #[test]
    fn duration_is_preserved() {
        let buffer = sine(44_100, 2, 0.5, 440.0);
        let output = resample(&buffer, 48_000).expect("resample");
        assert_eq!(output.sample_rate(), 48_000);
        assert_eq!(output.channels(), 2);
        let expected = 48_000.0 * 0.5;
        assert!((output.frames() as f64 - expected).abs() <= 2.0);
        assert!(output.peak() > 0.2);
    }
}
