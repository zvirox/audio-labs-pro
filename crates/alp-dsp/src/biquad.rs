//! Biquad filter implementations based on Robert Bristow-Johnson's Audio EQ Cookbook.
//!
//! Provides clean, real-time-safe Direct Form biquad filters for standard shapes:
//! LowPass, HighPass, BandPass, Notch, Peaking, LowShelf, and HighShelf.

use std::f32::consts::PI;

/// Shape of a biquad filter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterType {
    /// High-pass filter (cuts below cutoff).
    HighPass,
    /// Low-pass filter (cuts above cutoff).
    LowPass,
    /// Peaking bell filter with adjustable Q and gain in dB.
    Peaking,
    /// Low-frequency shelf with gain in dB.
    LowShelf,
    /// High-frequency shelf with gain in dB.
    HighShelf,
    /// Notch filter (attenuates narrow band around frequency).
    Notch,
    /// Band-pass filter (passes narrow band around frequency).
    BandPass,
}

/// Normalized biquad filter coefficients: H(z) = (b0 + b1*z^-1 + b2*z^-2) / (1 + a1*z^-1 + a2*z^-2)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coefficients {
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
    pub a1: f32,
    pub a2: f32,
}

impl Coefficients {
    /// Identity / pass-through coefficients.
    #[must_use]
    pub fn identity() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }

    /// Compute coefficients for a specific filter shape.
    ///
    /// - `filter_type`: Shape to design.
    /// - `sample_rate`: Sampling frequency in Hz (> 0).
    /// - `frequency`: Center/cutoff frequency in Hz (clamped below Nyquist).
    /// - `q`: Quality factor / resonance (> 0, standard Butterworth is ~0.707).
    /// - `gain_db`: Gain in decibels (used for Peaking, LowShelf, HighShelf).
    #[must_use]
    pub fn design(
        filter_type: FilterType,
        sample_rate: u32,
        frequency: f32,
        q: f32,
        gain_db: f32,
    ) -> Self {
        if sample_rate == 0 {
            return Self::identity();
        }

        let nyquist = sample_rate as f32 * 0.5;
        let freq = frequency.clamp(10.0, nyquist * 0.99);
        let q = q.max(0.01);
        let omega = 2.0 * PI * (freq / sample_rate as f32);
        let (sin_omega, cos_omega) = omega.sin_cos();
        let alpha = sin_omega / (2.0 * q);
        let a = 10.0f32.powf(gain_db / 40.0); // sqrt of linear gain for shelving/peaking

        let (b0, b1, b2, a0, a1, a2) = match filter_type {
            FilterType::HighPass => {
                let b0 = (1.0 + cos_omega) * 0.5;
                let b1 = -(1.0 + cos_omega);
                let b2 = (1.0 + cos_omega) * 0.5;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::LowPass => {
                let b0 = (1.0 - cos_omega) * 0.5;
                let b1 = 1.0 - cos_omega;
                let b2 = (1.0 - cos_omega) * 0.5;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::Peaking => {
                let b0 = 1.0 + alpha * a;
                let b1 = -2.0 * cos_omega;
                let b2 = 1.0 - alpha * a;
                let a0 = 1.0 + alpha / a;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha / a;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::LowShelf => {
                let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
                let b0 = a * ((a + 1.0) - (a - 1.0) * cos_omega + two_sqrt_a_alpha);
                let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_omega);
                let b2 = a * ((a + 1.0) - (a - 1.0) * cos_omega - two_sqrt_a_alpha);
                let a0 = (a + 1.0) + (a - 1.0) * cos_omega + two_sqrt_a_alpha;
                let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_omega);
                let a2 = (a + 1.0) + (a - 1.0) * cos_omega - two_sqrt_a_alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighShelf => {
                let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
                let b0 = a * ((a + 1.0) + (a - 1.0) * cos_omega + two_sqrt_a_alpha);
                let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_omega);
                let b2 = a * ((a + 1.0) + (a - 1.0) * cos_omega - two_sqrt_a_alpha);
                let a0 = (a + 1.0) - (a - 1.0) * cos_omega + two_sqrt_a_alpha;
                let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_omega);
                let a2 = (a + 1.0) - (a - 1.0) * cos_omega - two_sqrt_a_alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::Notch => {
                let b0 = 1.0;
                let b1 = -2.0 * cos_omega;
                let b2 = 1.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::BandPass => {
                let b0 = alpha;
                let b1 = 0.0;
                let b2 = -alpha;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
        };

        let inv_a0 = 1.0 / a0;
        Self {
            b0: b0 * inv_a0,
            b1: b1 * inv_a0,
            b2: b2 * inv_a0,
            a1: a1 * inv_a0,
            a2: a2 * inv_a0,
        }
    }
}

/// Single-channel biquad filter state.
#[derive(Debug, Clone, Copy, Default)]
pub struct ChannelState {
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl ChannelState {
    /// Reset filter memory.
    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }

    /// Process one sample through the filter.
    #[inline]
    #[must_use]
    pub fn process(&mut self, sample: f32, coeffs: &Coefficients) -> f32 {
        let out = coeffs.b0 * sample + coeffs.b1 * self.x1 + coeffs.b2 * self.x2
            - coeffs.a1 * self.y1
            - coeffs.a2 * self.y2;

        self.x2 = self.x1;
        self.x1 = sample;
        self.y2 = self.y1;
        // Anti-denormal flushing
        self.y1 = if out.abs() < 1e-15 { 0.0 } else { out };
        self.y1
    }
}

/// Multi-channel biquad filter.
#[derive(Debug, Clone)]
pub struct Biquad {
    coeffs: Coefficients,
    states: Vec<ChannelState>,
    sample_rate: u32,
    channels: u16,
    filter_type: FilterType,
    frequency: f32,
    q: f32,
    gain_db: f32,
}

impl Biquad {
    /// Create a new biquad filter.
    #[must_use]
    pub fn new(filter_type: FilterType, frequency: f32, q: f32, gain_db: f32) -> Self {
        Self {
            coeffs: Coefficients::identity(),
            states: Vec::new(),
            sample_rate: 0,
            channels: 0,
            filter_type,
            frequency,
            q,
            gain_db,
        }
    }

    /// Configure for a specific sample rate and channel layout.
    pub fn prepare(&mut self, sample_rate: u32, channels: u16) {
        self.sample_rate = sample_rate;
        self.channels = channels.max(1);
        self.states = vec![ChannelState::default(); self.channels as usize];
        self.update_coefficients();
    }

    /// Update filter parameters.
    pub fn set_parameters(&mut self, frequency: f32, q: f32, gain_db: f32) {
        self.frequency = frequency;
        self.q = q;
        self.gain_db = gain_db;
        self.update_coefficients();
    }

    /// Reset internal state buffers.
    pub fn reset(&mut self) {
        for state in &mut self.states {
            state.reset();
        }
    }

    /// Process interleaved audio buffer in place.
    pub fn process(&mut self, buffer: &mut [f32]) {
        let channels = self.channels as usize;
        if channels == 0 || self.states.len() < channels {
            return;
        }

        let mut ch = 0;
        for sample in buffer {
            *sample = self.states[ch].process(*sample, &self.coeffs);
            ch = (ch + 1) % channels;
        }
    }

    fn update_coefficients(&mut self) {
        if self.sample_rate > 0 {
            self.coeffs = Coefficients::design(
                self.filter_type,
                self.sample_rate,
                self.frequency,
                self.q,
                self.gain_db,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_pass_attenuates_low_frequencies() {
        let sample_rate = 48_000;
        let mut filter = Biquad::new(FilterType::HighPass, 500.0, 0.707, 0.0);
        filter.prepare(sample_rate, 1);

        // Generate 50 Hz tone (below cutoff)
        let mut low_buffer: Vec<f32> = (0..960)
            .map(|i| (2.0 * PI * 50.0 * (i as f32) / 48000.0).sin())
            .collect();
        filter.process(&mut low_buffer);
        let low_peak = low_buffer.iter().fold(0.0f32, |acc, &s| acc.max(s.abs()));

        // Low frequency should be significantly attenuated
        assert!(low_peak < 0.2, "expected attenuation, got {low_peak}");
    }

    #[test]
    fn low_pass_passes_low_frequencies() {
        let sample_rate = 48_000;
        let mut filter = Biquad::new(FilterType::LowPass, 1000.0, 0.707, 0.0);
        filter.prepare(sample_rate, 1);

        // Generate 100 Hz tone (well below cutoff)
        let mut low_buffer: Vec<f32> = (0..960)
            .map(|i| (2.0 * PI * 100.0 * (i as f32) / 48000.0).sin())
            .collect();
        filter.process(&mut low_buffer);
        let low_peak = low_buffer[480..]
            .iter()
            .fold(0.0f32, |acc, &s| acc.max(s.abs()));

        // Passband peak should be close to 1.0
        assert!(low_peak > 0.85, "expected pass-through, got {low_peak}");
    }
}
