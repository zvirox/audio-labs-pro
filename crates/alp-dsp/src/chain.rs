//! Composite effect chains and the One-Click Voice Cleanup preset.

use crate::Effect;
use crate::biquad::FilterType;
use crate::dynamics::{Compressor, NoiseGate};
use crate::eq::{EqBand, ParametricEq};
use crate::limiter::Limiter;
use crate::voice::{DeClick, DeEsser, DeHum};

/// A sequential pipeline of audio effects.
pub struct EffectChain {
    effects: Vec<Box<dyn Effect>>,
}

impl Default for EffectChain {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectChain {
    /// Create an empty effect chain.
    #[must_use]
    pub fn new() -> Self {
        Self {
            effects: Vec::new(),
        }
    }

    /// Add an effect to the end of the chain.
    pub fn add(&mut self, effect: Box<dyn Effect>) {
        self.effects.push(effect);
    }

    /// Number of active effects in the chain.
    #[must_use]
    pub fn len(&self) -> usize {
        self.effects.len()
    }

    /// Whether the chain is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }
}

impl Effect for EffectChain {
    fn name(&self) -> &'static str {
        "chain"
    }

    fn prepare(&mut self, sample_rate: u32, channels: u16) {
        for effect in &mut self.effects {
            effect.prepare(sample_rate, channels);
        }
    }

    fn process(&mut self, buffer: &mut [f32]) {
        for effect in &mut self.effects {
            effect.process(buffer);
        }
    }

    fn reset(&mut self) {
        for effect in &mut self.effects {
            effect.reset();
        }
    }
}

/// The One-Click Voice Cleanup preset chain.
///
/// Runs an optimized vocal sequence:
/// 1. High-Pass Filter (80 Hz) - cuts handling thumps and HVAC rumble
/// 2. De-Hum (50/60 Hz) - removes powerline mains noise
/// 3. De-Click - repairs mouth clicks and transients
/// 4. Noise Gate - suppresses room ambience during pauses
/// 5. Parametric EQ - gentle boxiness scoop and vocal presence lift
/// 6. De-Esser - attenuates harsh sibilance
/// 7. Vocal Compressor - evens out vocal dynamics
/// 8. True Peak Limiter - guarantees ceiling at -1 dBFS
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceCleanup;

impl VoiceCleanup {
    /// Build a new complete Voice Cleanup effect chain.
    #[must_use]
    pub fn build(mains_frequency: f32) -> EffectChain {
        let mut chain = EffectChain::new();

        // 1. High-pass filter & vocal EQ
        let eq_bands = vec![
            EqBand::new(FilterType::HighPass, 80.0, 0.707, 0.0),
            EqBand::new(FilterType::Peaking, 450.0, 1.2, -2.5), // reduce boxiness
            EqBand::new(FilterType::Peaking, 3500.0, 1.0, 2.0), // presence & clarity
            EqBand::new(FilterType::HighShelf, 10000.0, 0.707, 1.5), // air
        ];
        chain.add(Box::new(ParametricEq::with_bands(eq_bands)));

        // 2. De-Hum
        chain.add(Box::new(DeHum::new(mains_frequency)));

        // 3. De-Click
        chain.add(Box::new(DeClick::new(0.6)));

        // 4. Noise Gate
        chain.add(Box::new(NoiseGate::voice_default()));

        // 5. De-Esser
        chain.add(Box::new(DeEsser::voice_default()));

        // 6. Vocal Compressor
        chain.add(Box::new(Compressor::voice_default()));

        // 7. Output Limiter
        chain.add(Box::new(
            Limiter::new(-1.0, 1.0, 80.0).expect("valid limiter parameters"),
        ));

        chain
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_cleanup_chain_processes_audio_safely() {
        let mut cleanup = VoiceCleanup::build(50.0);
        cleanup.prepare(48_000, 1);

        let mut buffer = [0.25f32; 1000];
        cleanup.process(&mut buffer);

        for &sample in &buffer {
            assert!(sample.is_finite());
            assert!(sample.abs() <= 1.0);
        }
    }
}
