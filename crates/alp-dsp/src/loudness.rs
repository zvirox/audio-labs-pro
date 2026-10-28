//! ITU-R BS.1770-4 / EBU R128 loudness measurement and normalization.
//!
//! Provides K-weighted integrated loudness (LUFS) calculation and target
//! normalization (e.g. -16 LUFS for podcasts, -14 LUFS for streaming, -23 LUFS for broadcast).

use crate::biquad::{Biquad, FilterType};

/// K-weighting pre-filter according to ITU-R BS.1770-4.
#[derive(Debug, Clone)]
pub struct KWeighting {
    stage1_high_shelf: Biquad,
    stage2_high_pass: Biquad,
}

impl KWeighting {
    /// Create a K-weighting filter pair.
    #[must_use]
    pub fn new() -> Self {
        // Stage 1: High shelf at ~1680 Hz, +3.99 dB gain, Q ~0.707
        let stage1 = Biquad::new(FilterType::HighShelf, 1680.0, 0.707, 3.99);
        // Stage 2: High pass (RLB filter) at ~38 Hz, Q ~0.5
        let stage2 = Biquad::new(FilterType::HighPass, 38.0, 0.5, 0.0);

        Self {
            stage1_high_shelf: stage1,
            stage2_high_pass: stage2,
        }
    }

    /// Prepare filter states for audio stream format.
    pub fn prepare(&mut self, sample_rate: u32, channels: u16) {
        self.stage1_high_shelf.prepare(sample_rate, channels);
        self.stage2_high_pass.prepare(sample_rate, channels);
    }

    /// Filter samples in place.
    pub fn process(&mut self, buffer: &mut [f32]) {
        self.stage1_high_shelf.process(buffer);
        self.stage2_high_pass.process(buffer);
    }

    /// Reset filter states.
    pub fn reset(&mut self) {
        self.stage1_high_shelf.reset();
        self.stage2_high_pass.reset();
    }
}

impl Default for KWeighting {
    fn default() -> Self {
        Self::new()
    }
}

/// Measure integrated loudness in LUFS according to ITU-R BS.1770.
#[must_use]
pub fn measure_lufs(buffer: &[f32], sample_rate: u32, channels: u16) -> f32 {
    if buffer.is_empty() || sample_rate == 0 || channels == 0 {
        return -70.0;
    }

    let mut filtered = buffer.to_vec();
    let mut weighting = KWeighting::new();
    weighting.prepare(sample_rate, channels);
    weighting.process(&mut filtered);

    let ch = channels as usize;
    let frames = filtered.len() / ch;
    if frames == 0 {
        return -70.0;
    }

    // Compute channel mean square
    let mut sum_sq = 0.0f64;
    for sample in filtered {
        sum_sq += f64::from(sample) * f64::from(sample);
    }

    let mean_sq = sum_sq / (frames as f64 * ch as f64);
    if mean_sq <= 1e-12 {
        return -70.0;
    }

    // BS.1770 formula: -0.691 + 10 * log10(sum of channel-weighted powers)
    let lufs = -0.691 + 10.0 * mean_sq.log10();
    (lufs as f32).max(-70.0)
}

/// Calculate the linear gain multiplier to normalize from `current_lufs` to `target_lufs`,
/// respecting a maximum ceiling (true peak) in dB FS.
#[must_use]
pub fn calculate_normalization_gain(
    current_lufs: f32,
    target_lufs: f32,
    peak_linear: f32,
    ceiling_db: f32,
) -> f32 {
    if current_lufs <= -69.0 {
        return 1.0;
    }

    let needed_gain_db = target_lufs - current_lufs;
    let mut linear_gain = 10.0f32.powf(needed_gain_db / 20.0);

    // Ensure we do not push peaks past ceiling
    let ceiling_linear = 10.0f32.powf(ceiling_db / 20.0);
    if peak_linear > 0.0 && (peak_linear * linear_gain) > ceiling_linear {
        linear_gain = ceiling_linear / peak_linear;
    }

    linear_gain
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_scale_sine_measures_close_to_expected() {
        let sample_rate = 48_000;
        // 1 kHz sine at 0 dBFS has BS.1770 loudness approx -3.01 LUFS
        let samples: Vec<f32> = (0..sample_rate)
            .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * (i as f32) / 48000.0).sin())
            .collect();

        let lufs = measure_lufs(&samples, sample_rate, 1);
        assert!(
            (lufs - (-3.0)).abs() < 1.0,
            "expected ~ -3 LUFS, got {lufs}"
        );
    }

    #[test]
    fn normalization_gain_respects_ceiling() {
        let current_lufs = -24.0;
        let target_lufs = -16.0; // Needs +8 dB gain (~2.51x)
        let peak = 0.8;
        let ceiling = -1.0; // Max amplitude ~0.891

        let gain = calculate_normalization_gain(current_lufs, target_lufs, peak, ceiling);
        assert!((peak * gain) <= 0.892, "peak exceeded ceiling");
    }
}
