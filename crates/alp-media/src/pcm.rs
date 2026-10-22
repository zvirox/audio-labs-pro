//! Interleaved float PCM buffers.
//!
//! This is the internal audio currency between decode, resample, mixing and
//! encoding: f32 samples, interleaved, at a known sample rate and channel count
//! (ADR 0004).

use alp_core::{MAX_SAMPLE_RATE, MIN_SAMPLE_RATE};

use crate::error::MediaError;

/// Interleaved f32 audio.
#[derive(Debug, Clone, PartialEq)]
pub struct PcmBuffer {
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
}

impl PcmBuffer {
    /// Build a buffer from interleaved samples.
    pub fn new(sample_rate: u32, channels: u16, samples: Vec<f32>) -> Result<Self, MediaError> {
        if !(MIN_SAMPLE_RATE..=MAX_SAMPLE_RATE).contains(&sample_rate) {
            return Err(MediaError::InvalidBuffer(format!(
                "unsupported sample rate {sample_rate}"
            )));
        }
        if channels == 0 {
            return Err(MediaError::InvalidBuffer("zero channels".into()));
        }
        let channels_usize = usize::from(channels);
        if !samples.len().is_multiple_of(channels_usize) {
            return Err(MediaError::InvalidBuffer(format!(
                "{} samples is not a multiple of {channels} channels",
                samples.len()
            )));
        }
        Ok(Self {
            sample_rate,
            channels,
            samples,
        })
    }

    /// Create a silent buffer of `frames` frames.
    pub fn silence(sample_rate: u32, channels: u16, frames: u64) -> Result<Self, MediaError> {
        let len = frames
            .checked_mul(u64::from(channels))
            .ok_or_else(|| MediaError::InvalidBuffer("frame count overflow".into()))?;
        let len = usize::try_from(len)
            .map_err(|_| MediaError::InvalidBuffer("frame count too large".into()))?;
        Self::new(sample_rate, channels, vec![0.0; len])
    }

    /// Sample rate in Hz.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Channel count.
    #[must_use]
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Number of frames.
    #[must_use]
    pub fn frames(&self) -> u64 {
        self.samples.len() as u64 / u64::from(self.channels)
    }

    /// Duration in seconds.
    #[must_use]
    pub fn duration_secs(&self) -> f64 {
        self.frames() as f64 / f64::from(self.sample_rate)
    }

    /// Interleaved samples.
    #[must_use]
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    /// Interleaved samples, mutably.
    #[must_use]
    pub fn samples_mut(&mut self) -> &mut [f32] {
        &mut self.samples
    }

    /// Consume the buffer and return its samples.
    #[must_use]
    pub fn into_samples(self) -> Vec<f32> {
        self.samples
    }

    /// Sample at a frame and channel, if both are in range.
    #[must_use]
    pub fn sample(&self, frame: u64, channel: u16) -> Option<f32> {
        if channel >= self.channels || frame >= self.frames() {
            return None;
        }
        let index = frame as usize * usize::from(self.channels) + usize::from(channel);
        self.samples.get(index).copied()
    }

    /// Replace every sample with its negative.
    pub fn invert(&mut self) {
        for sample in &mut self.samples {
            *sample = -*sample;
        }
    }

    /// Largest absolute sample value.
    #[must_use]
    pub fn peak(&self) -> f32 {
        self.samples
            .iter()
            .fold(0.0_f32, |peak, sample| peak.max(sample.abs()))
    }

    /// Root-mean-square level.
    #[must_use]
    pub fn rms(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let sum: f64 = self
            .samples
            .iter()
            .map(|sample| f64::from(*sample) * f64::from(*sample))
            .sum();
        (sum / self.samples.len() as f64).sqrt() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimensions_are_validated() {
        assert!(PcmBuffer::new(48_000, 0, vec![]).is_err());
        assert!(PcmBuffer::new(48_000, 2, vec![0.0; 3]).is_err());
        assert!(PcmBuffer::new(1, 1, vec![]).is_err());
        let buffer = PcmBuffer::new(48_000, 2, vec![0.0; 8]).expect("buffer");
        assert_eq!(buffer.frames(), 4);
        assert_eq!(buffer.duration_secs(), 4.0 / 48_000.0);
    }

    #[test]
    fn peak_and_rms_are_measured() {
        let buffer = PcmBuffer::new(48_000, 1, vec![0.5, -0.5, 1.0, -1.0]).expect("buffer");
        assert!((buffer.peak() - 1.0).abs() < 1e-6);
        assert!((buffer.rms() - 0.790_569_6).abs() < 1e-5);
        assert!(PcmBuffer::silence(48_000, 2, 4).is_ok());
    }
}
