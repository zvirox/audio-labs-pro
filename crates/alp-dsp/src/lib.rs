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
}
