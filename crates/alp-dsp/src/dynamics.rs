//! Dynamics processors: Compressor and Noise Gate.
//!
//! Provides vocal dynamic range control, levelling, and background noise gating.

use alp_core::CoreError;

use crate::Effect;

/// Dynamic range compressor with soft-knee and makeup gain.
#[derive(Debug, Clone)]
pub struct Compressor {
    threshold_db: f32,
    ratio: f32,
    attack_ms: f32,
    release_ms: f32,
    knee_db: f32,
    makeup_db: f32,
    envelope: f32,
    sample_rate: u32,
    channels: u16,
}

impl Compressor {
    /// Create a new compressor.
    ///
    /// - `threshold_db`: Threshold in dB FS (typically -30.0 .. 0.0)
    /// - `ratio`: Compression ratio >= 1.0 (e.g. 2.0 to 8.0 for voice)
    /// - `attack_ms`: Attack time in ms (> 0.0, e.g. 5.0 to 20.0 ms)
    /// - `release_ms`: Release time in ms (> 0.0, e.g. 50.0 to 250.0 ms)
    /// - `knee_db`: Soft knee width in dB (>= 0.0, e.g. 3.0 to 6.0 dB)
    /// - `makeup_db`: Output makeup gain in dB (>= 0.0)
    pub fn new(
        threshold_db: f32,
        ratio: f32,
        attack_ms: f32,
        release_ms: f32,
        knee_db: f32,
        makeup_db: f32,
    ) -> Result<Self, CoreError> {
        if !threshold_db.is_finite()
            || !ratio.is_finite()
            || !attack_ms.is_finite()
            || !release_ms.is_finite()
            || !knee_db.is_finite()
            || !makeup_db.is_finite()
            || ratio < 1.0
            || attack_ms <= 0.0
            || release_ms <= 0.0
            || knee_db < 0.0
        {
            return Err(CoreError::validation("invalid compressor parameters"));
        }

        Ok(Self {
            threshold_db,
            ratio,
            attack_ms,
            release_ms,
            knee_db,
            makeup_db,
            envelope: 0.0,
            sample_rate: 0,
            channels: 0,
        })
    }

    /// Pre-configured natural vocal compressor.
    #[must_use]
    pub fn voice_default() -> Self {
        Self::new(-18.0, 3.0, 10.0, 100.0, 4.0, 2.0).expect("valid parameters")
    }

    fn alpha(time_ms: f32, sample_rate: u32) -> f32 {
        if time_ms <= 0.0 || sample_rate == 0 {
            return 1.0;
        }
        (-1.0 / (time_ms * 0.001 * sample_rate as f32)).exp()
    }
}

impl Effect for Compressor {
    fn name(&self) -> &'static str {
        "compressor"
    }

    fn prepare(&mut self, sample_rate: u32, channels: u16) {
        self.sample_rate = sample_rate;
        self.channels = channels.max(1);
        self.envelope = 0.0;
    }

    fn process(&mut self, buffer: &mut [f32]) {
        let channels = self.channels as usize;
        if channels == 0 || self.sample_rate == 0 {
            return;
        }

        let attack_coeff = Self::alpha(self.attack_ms, self.sample_rate);
        let release_coeff = Self::alpha(self.release_ms, self.sample_rate);
        let makeup_lin = 10.0f32.powf(self.makeup_db / 20.0);
        let half_knee = self.knee_db * 0.5;

        let mut index = 0;
        while index < buffer.len() {
            // Find frame peak for sidechain
            let mut peak = 1e-6f32;
            for ch in 0..channels {
                if index + ch < buffer.len() {
                    peak = peak.max(buffer[index + ch].abs());
                }
            }

            let input_db = 20.0 * peak.log10();

            // Compute static compression curve with soft knee
            let target_gain_db = if input_db < (self.threshold_db - half_knee) {
                0.0
            } else if half_knee > 0.0 && input_db <= (self.threshold_db + half_knee) {
                let delta = input_db - self.threshold_db + half_knee;
                ((1.0 / self.ratio) - 1.0) * (delta * delta) / (2.0 * self.knee_db)
            } else {
                (self.threshold_db + (input_db - self.threshold_db) / self.ratio) - input_db
            };

            // Convert target gain reduction to positive attenuation in dB
            let target_reduction = -target_gain_db.min(0.0);

            // Ballistics smoothing
            let coeff = if target_reduction > self.envelope {
                attack_coeff
            } else {
                release_coeff
            };
            self.envelope = target_reduction + coeff * (self.envelope - target_reduction);

            let gain_mult = 10.0f32.powf(-self.envelope / 20.0) * makeup_lin;

            for ch in 0..channels {
                if index + ch < buffer.len() {
                    buffer[index + ch] *= gain_mult;
                }
            }

            index += channels;
        }
    }

    fn reset(&mut self) {
        self.envelope = 0.0;
    }
}

/// Noise gate / downward expander for voice and dialogue.
#[derive(Debug, Clone)]
pub struct NoiseGate {
    threshold_db: f32,
    attack_ms: f32,
    hold_ms: f32,
    release_ms: f32,
    attenuation_db: f32,
    gain: f32,
    hold_samples: u32,
    hold_counter: u32,
    sample_rate: u32,
    channels: u16,
}

impl NoiseGate {
    /// Create a new noise gate.
    ///
    /// - `threshold_db`: Open threshold in dB FS (e.g. -45.0 to -30.0 dB)
    /// - `attack_ms`: Opening attack time in ms (e.g. 2.0 to 10.0 ms)
    /// - `hold_ms`: Time gate stays open after speech falls below threshold (e.g. 50.0 to 150.0 ms)
    /// - `release_ms`: Closing time in ms (e.g. 100.0 to 300.0 ms)
    /// - `attenuation_db`: Maximum attenuation when closed (e.g. -40.0 dB to -80.0 dB)
    pub fn new(
        threshold_db: f32,
        attack_ms: f32,
        hold_ms: f32,
        release_ms: f32,
        attenuation_db: f32,
    ) -> Result<Self, CoreError> {
        if !threshold_db.is_finite()
            || !attack_ms.is_finite()
            || !hold_ms.is_finite()
            || !release_ms.is_finite()
            || !attenuation_db.is_finite()
            || attack_ms <= 0.0
            || hold_ms < 0.0
            || release_ms <= 0.0
        {
            return Err(CoreError::validation("invalid noise gate parameters"));
        }

        Ok(Self {
            threshold_db,
            attack_ms,
            hold_ms,
            release_ms,
            attenuation_db,
            gain: 0.0, // starts closed
            hold_samples: 0,
            hold_counter: 0,
            sample_rate: 0,
            channels: 0,
        })
    }

    /// Pre-configured natural vocal noise gate.
    #[must_use]
    pub fn voice_default() -> Self {
        Self::new(-38.0, 2.0, 80.0, 150.0, -40.0).expect("valid parameters")
    }

    fn coeff(ms: f32, sample_rate: u32) -> f32 {
        if ms <= 0.0 || sample_rate == 0 {
            return 1.0;
        }
        1.0 - (-1.0 / (ms * 0.001 * sample_rate as f32)).exp()
    }
}

impl Effect for NoiseGate {
    fn name(&self) -> &'static str {
        "gate"
    }

    fn prepare(&mut self, sample_rate: u32, channels: u16) {
        self.sample_rate = sample_rate;
        self.channels = channels.max(1);
        self.hold_samples = (self.hold_ms * 0.001 * sample_rate as f32) as u32;
        self.hold_counter = 0;
        self.gain = 10.0f32.powf(self.attenuation_db / 20.0);
    }

    fn process(&mut self, buffer: &mut [f32]) {
        let channels = self.channels as usize;
        if channels == 0 || self.sample_rate == 0 {
            return;
        }

        let min_gain = 10.0f32.powf(self.attenuation_db / 20.0);
        let attack_step = Self::coeff(self.attack_ms, self.sample_rate);
        let release_step = Self::coeff(self.release_ms, self.sample_rate);

        let mut index = 0;
        while index < buffer.len() {
            let mut peak = 1e-6f32;
            for ch in 0..channels {
                if index + ch < buffer.len() {
                    peak = peak.max(buffer[index + ch].abs());
                }
            }

            let input_db = 20.0 * peak.log10();

            let target_gain = if input_db >= self.threshold_db {
                self.hold_counter = self.hold_samples;
                1.0
            } else if self.hold_counter > 0 {
                self.hold_counter -= 1;
                1.0
            } else {
                min_gain
            };

            // Smooth transition
            if target_gain > self.gain {
                self.gain += attack_step * (target_gain - self.gain);
            } else {
                self.gain += release_step * (target_gain - self.gain);
            }

            for ch in 0..channels {
                if index + ch < buffer.len() {
                    buffer[index + ch] *= self.gain;
                }
            }

            index += channels;
        }
    }

    fn reset(&mut self) {
        self.hold_counter = 0;
        self.gain = 10.0f32.powf(self.attenuation_db / 20.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compressor_reduces_loud_signals() {
        let mut comp = Compressor::new(-10.0, 4.0, 1.0, 50.0, 0.0, 0.0).expect("compressor");
        comp.prepare(48_000, 1);

        // Loud signal at 0 dB (amplitude 1.0)
        let mut buffer = [1.0f32; 480];
        comp.process(&mut buffer);

        let end_peak = buffer[400..].iter().fold(0.0f32, |m, &s| m.max(s.abs()));
        // Above -10 dB threshold with 4:1 ratio, gain reduction must be evident
        assert!(end_peak < 0.8, "expected compression, got {end_peak}");
    }

    #[test]
    fn noise_gate_attenuates_quiet_signals() {
        let mut gate = NoiseGate::new(-30.0, 1.0, 0.0, 10.0, -40.0).expect("gate");
        gate.prepare(48_000, 1);

        // Very quiet signal at -50 dB (amplitude ~0.003)
        let mut buffer = [0.003f32; 1000];
        gate.process(&mut buffer);

        let end_sample = buffer[950].abs();
        assert!(
            end_sample < 0.0005,
            "expected gate attenuation, got {end_sample}"
        );
    }
}
