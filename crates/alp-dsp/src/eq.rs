//! Multi-band parametric equalizer.
//!
//! Features high-pass filtering for rumble removal, low/high shelving for tonal
//! balance, and dual parametric peaking bands for resonance control and presence.

use crate::Effect;
use crate::biquad::{Biquad, FilterType};

/// A single parametric EQ band configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct EqBand {
    /// Filter shape.
    pub filter_type: FilterType,
    /// Center or cutoff frequency in Hz.
    pub frequency: f32,
    /// Quality factor Q.
    pub q: f32,
    /// Gain in decibels.
    pub gain_db: f32,
    /// Whether this band is enabled.
    pub enabled: bool,
}

impl EqBand {
    /// Create an enabled band.
    #[must_use]
    pub fn new(filter_type: FilterType, frequency: f32, q: f32, gain_db: f32) -> Self {
        Self {
            filter_type,
            frequency,
            q,
            gain_db,
            enabled: true,
        }
    }
}

/// 5-band studio parametric equalizer.
#[derive(Debug, Clone)]
pub struct ParametricEq {
    bands: Vec<(EqBand, Biquad)>,
    sample_rate: u32,
    channels: u16,
}

impl ParametricEq {
    /// Create a standard 5-band voice EQ:
    /// - High-Pass: 80 Hz (cuts rumble/plosives)
    /// - Low Shelf: 200 Hz
    /// - Low-Mid Bell: 800 Hz
    /// - High-Mid Bell: 3.5 kHz
    /// - High Shelf: 10 kHz (air)
    #[must_use]
    pub fn voice_default() -> Self {
        let default_bands = vec![
            EqBand::new(FilterType::HighPass, 80.0, 0.707, 0.0),
            EqBand::new(FilterType::LowShelf, 200.0, 0.707, 0.0),
            EqBand::new(FilterType::Peaking, 800.0, 1.0, 0.0),
            EqBand::new(FilterType::Peaking, 3500.0, 1.2, 0.0),
            EqBand::new(FilterType::HighShelf, 10000.0, 0.707, 0.0),
        ];
        Self::with_bands(default_bands)
    }

    /// Construct an EQ with custom bands.
    #[must_use]
    pub fn with_bands(bands: Vec<EqBand>) -> Self {
        let biquads = bands
            .into_iter()
            .map(|b| {
                let filter = Biquad::new(b.filter_type, b.frequency, b.q, b.gain_db);
                (b, filter)
            })
            .collect();

        Self {
            bands: biquads,
            sample_rate: 0,
            channels: 0,
        }
    }

    /// Update band parameters at `index`.
    pub fn set_band(&mut self, index: usize, frequency: f32, q: f32, gain_db: f32) {
        if let Some((band, filter)) = self.bands.get_mut(index) {
            band.frequency = frequency;
            band.q = q;
            band.gain_db = gain_db;
            filter.set_parameters(frequency, q, gain_db);
        }
    }

    /// Enable or disable a band at `index`.
    pub fn set_band_enabled(&mut self, index: usize, enabled: bool) {
        if let Some((band, _)) = self.bands.get_mut(index) {
            band.enabled = enabled;
        }
    }
}

impl Effect for ParametricEq {
    fn name(&self) -> &'static str {
        "eq"
    }

    fn prepare(&mut self, sample_rate: u32, channels: u16) {
        self.sample_rate = sample_rate;
        self.channels = channels.max(1);
        for (_, filter) in &mut self.bands {
            filter.prepare(sample_rate, channels);
        }
    }

    fn process(&mut self, buffer: &mut [f32]) {
        for (band, filter) in &mut self.bands {
            if band.enabled {
                filter.process(buffer);
            }
        }
    }

    fn reset(&mut self) {
        for (_, filter) in &mut self.bands {
            filter.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eq_runs_and_processes_audio() {
        let mut eq = ParametricEq::voice_default();
        eq.prepare(48_000, 2);

        // Boost high shelf by +6 dB
        eq.set_band(4, 10000.0, 0.707, 6.0);

        let mut buffer = [0.1f32; 256];
        eq.process(&mut buffer);

        // Processed audio should not NaN or blow up
        for sample in buffer {
            assert!(sample.is_finite());
        }
    }
}
