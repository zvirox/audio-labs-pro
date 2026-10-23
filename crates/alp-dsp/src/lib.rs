//! `alp-dsp` — audio effects and metering.
//!
//! Phase 1 provides the `Effect` boundary and a reference gain stage. EQ,
//! dynamics, denoise and loudness processors land in Phase 5.

use alp_core::CoreError;

/// An audio processor that can run live or offline.
pub trait Effect: Send {
    /// Stable effect name used by projects and presets.
    fn name(&self) -> &'static str;

    /// Prepare internal state for a stream format.
    fn prepare(&mut self, sample_rate: u32, channels: u16);

    /// Process interleaved f32 samples in place.
    fn process(&mut self, buffer: &mut [f32]);

    /// Reset delay lines and envelopes.
    fn reset(&mut self);
}

/// Linear gain stage, mostly useful as a boundary test and a fallback.
#[derive(Debug, Clone)]
pub struct Gain {
    gain: f32,
}

/// A simple peak limiter.
#[derive(Debug, Clone)]
pub struct Limiter {
    threshold: f32,
    _attack_ms: f32,
    release_ms: f32,
    sample_rate: u32,
    channels: u16,
    gain: f32,
}

impl Limiter {
    /// Build a limiter with a threshold in dB FS, attack and release in
    /// milliseconds.
    pub fn new(threshold_db: f32, attack_ms: f32, release_ms: f32) -> Result<Self, CoreError> {
        if !threshold_db.is_finite()
            || !attack_ms.is_finite()
            || !release_ms.is_finite()
            || attack_ms < 0.0
            || release_ms < 0.0
        {
            return Err(CoreError::validation("invalid limiter parameter"));
        }
        Ok(Self {
            threshold: 10f32.powf(threshold_db / 20.0),
            _attack_ms: attack_ms,
            release_ms,
            sample_rate: 0,
            channels: 0,
            gain: 1.0,
        })
    }

    #[must_use]
    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    fn coefficient(ms: f32, sample_rate: u32) -> f32 {
        if ms <= 0.0 || sample_rate == 0 {
            return 1.0;
        }
        (1.0 - (-1.0 / (ms * 0.001 * sample_rate as f32)).exp()).clamp(0.0, 1.0)
    }
}

impl Effect for Limiter {
    fn name(&self) -> &'static str {
        "limiter"
    }

    fn prepare(&mut self, sample_rate: u32, channels: u16) {
        self.sample_rate = sample_rate;
        self.channels = channels.max(1);
        self.gain = 1.0;
    }

    fn process(&mut self, buffer: &mut [f32]) {
        let channels = self.channels.max(1) as usize;
        let mut index = 0;
        while index < buffer.len() {
            let mut peak = 0.0f32;
            for channel in 0..channels {
                if index + channel < buffer.len() {
                    peak = peak.max(buffer[index + channel].abs());
                }
            }
            let target_gain = if peak > self.threshold {
                self.threshold / peak
            } else {
                1.0
            };
            // Attenuate immediately to honour a fast attack, but release smoothly.
            if target_gain < self.gain {
                self.gain = target_gain;
            } else {
                let coeff = Self::coefficient(self.release_ms, self.sample_rate);
                self.gain = target_gain * coeff + self.gain * (1.0 - coeff);
            }
            for channel in 0..channels {
                if index + channel < buffer.len() {
                    buffer[index + channel] *= self.gain;
                }
            }
            index += channels;
        }
    }

    fn reset(&mut self) {
        self.gain = 1.0;
    }
}

impl Gain {
    /// Build a gain stage. Rejects non-finite or negative values.
    pub fn new(gain: f32) -> Result<Self, CoreError> {
        if !gain.is_finite() || gain < 0.0 {
            return Err(CoreError::validation(
                "gain must be finite and non-negative",
            ));
        }
        Ok(Self { gain })
    }

    /// Current gain multiplier.
    #[must_use]
    pub fn gain(&self) -> f32 {
        self.gain
    }
}

impl Effect for Gain {
    fn name(&self) -> &'static str {
        "gain"
    }

    fn prepare(&mut self, _sample_rate: u32, _channels: u16) {}

    fn process(&mut self, buffer: &mut [f32]) {
        for sample in buffer {
            *sample *= self.gain;
        }
    }

    fn reset(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gain_scales_samples() {
        let mut effect = Gain::new(0.5).expect("gain");
        let mut buffer = [1.0, -1.0, 0.25];
        effect.process(&mut buffer);
        assert_eq!(buffer, [0.5, -0.5, 0.125]);
    }

    #[test]
    fn limiter_reduces_peaks_above_threshold() {
        let mut limiter = Limiter::new(-12.0, 1.0, 100.0).expect("limiter");
        limiter.prepare(48_000, 1);
        let mut buffer = [1.0, 1.0, 0.5];
        limiter.process(&mut buffer);
        for sample in buffer {
            assert!(sample.abs() <= 0.26, "sample {}", sample);
        }
        assert!(limiter.threshold() < 1.0);
    }
}
