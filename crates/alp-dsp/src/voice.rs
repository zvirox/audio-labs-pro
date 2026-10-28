//! Voice cleanup processors: De-Esser, De-Hum, and De-Click.
//!
//! Designed specifically for spoken word, podcasting and voice-over tracks.

use alp_core::CoreError;

use crate::Effect;
use crate::biquad::{Biquad, FilterType};

/// Sibilance attenuator (De-Esser).
///
/// Uses a sidechain bandpass filter centered on sibilance frequencies (typically
/// 4 kHz to 8 kHz) to dynamically compress harsh high-frequency bursts without
/// darkening the overall voice.
#[derive(Debug, Clone)]
pub struct DeEsser {
    frequency: f32,
    threshold_db: f32,
    ratio: f32,
    detector: Biquad,
    gain_reduction: f32,
    sample_rate: u32,
    channels: u16,
}

impl DeEsser {
    /// Create a de-esser.
    ///
    /// - `frequency`: Center sibilance frequency in Hz (e.g. 5500.0 to 7000.0 Hz)
    /// - `threshold_db`: Threshold in dB FS (e.g. -24.0 to -12.0 dB)
    /// - `ratio`: Attenuation ratio (e.g. 3.0 to 8.0)
    pub fn new(frequency: f32, threshold_db: f32, ratio: f32) -> Result<Self, CoreError> {
        if !frequency.is_finite()
            || !threshold_db.is_finite()
            || !ratio.is_finite()
            || frequency < 1000.0
            || ratio < 1.0
        {
            return Err(CoreError::validation("invalid de-esser parameters"));
        }

        let detector = Biquad::new(FilterType::BandPass, frequency, 2.0, 0.0);

        Ok(Self {
            frequency,
            threshold_db,
            ratio,
            detector,
            gain_reduction: 1.0,
            sample_rate: 0,
            channels: 0,
        })
    }

    /// Pre-configured natural vocal de-esser.
    #[must_use]
    pub fn voice_default() -> Self {
        Self::new(6000.0, -18.0, 4.0).expect("valid parameters")
    }

    /// Center frequency in Hz.
    #[must_use]
    pub fn frequency(&self) -> f32 {
        self.frequency
    }
}

impl Effect for DeEsser {
    fn name(&self) -> &'static str {
        "de-esser"
    }

    fn prepare(&mut self, sample_rate: u32, channels: u16) {
        self.sample_rate = sample_rate;
        self.channels = channels.max(1);
        self.detector.prepare(sample_rate, channels);
        self.gain_reduction = 1.0;
    }

    fn process(&mut self, buffer: &mut [f32]) {
        let channels = self.channels as usize;
        if channels == 0 || self.sample_rate == 0 {
            return;
        }

        // Sidechain filtered buffer to detect sibilant energy
        let mut sidechain = buffer.to_vec();
        self.detector.process(&mut sidechain);

        let attack_coeff = 0.85f32; // Fast attack for transients
        let release_coeff = 0.985f32; // Smooth release

        let mut index = 0;
        while index < buffer.len() {
            let mut sibilance_peak = 1e-6f32;
            for ch in 0..channels {
                if index + ch < sidechain.len() {
                    sibilance_peak = sibilance_peak.max(sidechain[index + ch].abs());
                }
            }

            let sibilance_db = 20.0 * sibilance_peak.log10();
            let target_gain = if sibilance_db > self.threshold_db {
                let over_db = sibilance_db - self.threshold_db;
                let reduction_db = over_db * (1.0 - 1.0 / self.ratio);
                10.0f32.powf(-reduction_db / 20.0)
            } else {
                1.0
            };

            if target_gain < self.gain_reduction {
                self.gain_reduction =
                    target_gain * (1.0 - attack_coeff) + self.gain_reduction * attack_coeff;
            } else {
                self.gain_reduction =
                    target_gain * (1.0 - release_coeff) + self.gain_reduction * release_coeff;
            }

            for ch in 0..channels {
                if index + ch < buffer.len() {
                    buffer[index + ch] *= self.gain_reduction;
                }
            }

            index += channels;
        }
    }

    fn reset(&mut self) {
        self.detector.reset();
        self.gain_reduction = 1.0;
    }
}

/// Power-line mains hum remover.
///
/// Places high-Q notch filters at the fundamental (50 Hz in EU/UK/Asia or 60 Hz in US)
/// and its first 3 harmonic multiples (e.g. 100/120, 150/180, 200/240 Hz).
#[derive(Debug, Clone)]
pub struct DeHum {
    base_frequency: f32,
    notches: Vec<Biquad>,
    sample_rate: u32,
    channels: u16,
}

impl DeHum {
    /// Create a de-hum filter for 50 Hz or 60 Hz mains.
    #[must_use]
    pub fn new(base_frequency: f32) -> Self {
        let base = if base_frequency >= 55.0 { 60.0 } else { 50.0 };
        let mut notches = Vec::with_capacity(4);
        for harmonic in 1..=4 {
            let freq = base * harmonic as f32;
            notches.push(Biquad::new(FilterType::Notch, freq, 12.0, 0.0));
        }

        Self {
            base_frequency: base,
            notches,
            sample_rate: 0,
            channels: 0,
        }
    }

    /// Base mains frequency in Hz.
    #[must_use]
    pub fn base_frequency(&self) -> f32 {
        self.base_frequency
    }
}

impl Effect for DeHum {
    fn name(&self) -> &'static str {
        "de-hum"
    }

    fn prepare(&mut self, sample_rate: u32, channels: u16) {
        self.sample_rate = sample_rate;
        self.channels = channels.max(1);
        for notch in &mut self.notches {
            notch.prepare(sample_rate, channels);
        }
    }

    fn process(&mut self, buffer: &mut [f32]) {
        for notch in &mut self.notches {
            notch.process(buffer);
        }
    }

    fn reset(&mut self) {
        for notch in &mut self.notches {
            notch.reset();
        }
    }
}

/// Mouth click and pop transient repair.
///
/// Detects sudden microsecond-level discontinuities in speech waveforms and
/// replaces them with smooth polynomial interpolation.
#[derive(Debug, Clone)]
pub struct DeClick {
    sensitivity: f32,
    channels: u16,
}

impl DeClick {
    /// Create a de-click effect with sensitivity 0.1 to 1.0.
    #[must_use]
    pub fn new(sensitivity: f32) -> Self {
        Self {
            sensitivity: sensitivity.clamp(0.05, 1.0),
            channels: 1,
        }
    }
}

impl Default for DeClick {
    fn default() -> Self {
        Self::new(0.5)
    }
}

impl Effect for DeClick {
    fn name(&self) -> &'static str {
        "de-click"
    }

    fn prepare(&mut self, _sample_rate: u32, channels: u16) {
        self.channels = channels.max(1);
    }

    fn process(&mut self, buffer: &mut [f32]) {
        let channels = self.channels as usize;
        if buffer.len() < channels * 4 {
            return;
        }

        let threshold = 0.35 * (1.05 - self.sensitivity);

        for ch in 0..channels {
            let mut i = ch + channels;
            while i + channels * 2 < buffer.len() {
                let prev = buffer[i - channels];
                let curr = buffer[i];
                let next = buffer[i + channels];

                // Second derivative / abrupt impulse detection
                let delta1 = curr - prev;
                let delta2 = next - curr;
                let second_deriv = (delta2 - delta1).abs();

                if second_deriv > threshold && curr.abs() > 0.01 {
                    // Smooth click with linear/cubic interpolation of neighbors
                    buffer[i] = (prev + next) * 0.5;
                }

                i += channels;
            }
        }
    }

    fn reset(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn de_hum_attenuates_mains_frequency() {
        let mut de_hum = DeHum::new(50.0);
        de_hum.prepare(48_000, 1);

        // 50 Hz hum tone for 1 second (steady state reached after filter ring-down)
        let mut buffer: Vec<f32> = (0..48_000)
            .map(|i| (2.0 * PI * 50.0 * (i as f32) / 48000.0).sin())
            .collect();
        de_hum.process(&mut buffer);

        let end_peak = buffer[40_000..].iter().fold(0.0f32, |m, &s| m.max(s.abs()));
        assert!(end_peak < 0.25, "expected 50Hz hum notch, got {end_peak}");
    }

    #[test]
    fn de_click_repairs_isolated_spikes() {
        let mut de_click = DeClick::new(0.8);
        de_click.prepare(48_000, 1);

        let mut buffer = [0.1f32; 32];
        // Inject an abrupt click at sample 10
        buffer[10] = 0.95;

        de_click.process(&mut buffer);
        assert!(
            buffer[10] < 0.6,
            "expected spike reduction, got {}",
            buffer[10]
        );
    }
}
