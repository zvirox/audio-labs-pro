//! Per-track insert rack for live and offline mixing.
//!
//! A rack holds at most [`MAX_INSERT_SLOTS`] DSP inserts. Slot configuration is
//! built once from a project's `EffectRef` chain before playback starts; an
//! [`InsertRackControl`] handle shares atomics with the mixer so a UI/worker can
//! change bypass and parameters without touching the audio thread.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use alp_core::EffectRef;
use alp_dsp::{
    Compressor, DeClick, DeEsser, DeHum, Effect, FilterType, Gain, HarmonicSuppressor, Limiter,
    NoiseGate, ParametricEq, SpectralNotchFilter, EqBand,
};

/// Maximum insert slots per track.
pub const MAX_INSERT_SLOTS: usize = 8;
/// Parameter slots exposed per insert.
pub const INSERT_PARAM_COUNT: usize = 8;

type ParamAtomics = Arc<[[AtomicU32; INSERT_PARAM_COUNT]; MAX_INSERT_SLOTS]>;
type EnabledAtomics = Arc<[AtomicBool; MAX_INSERT_SLOTS]>;

/// Lock-free handle for changing a rack's live state from any thread.
#[derive(Clone)]
pub struct InsertRackControl {
    enabled: EnabledAtomics,
    params: ParamAtomics,
}

impl Default for InsertRackControl {
    fn default() -> Self {
        Self {
            enabled: Arc::new(std::array::from_fn(|_| AtomicBool::new(true))),
            params: Arc::new(std::array::from_fn(|_| {
                std::array::from_fn(|_| AtomicU32::new(0.0f32.to_bits()))
            })),
        }
    }
}

impl InsertRackControl {
    /// Bypass or unbypass an insert slot.
    pub fn set_enabled(&self, slot: usize, enabled: bool) {
        if let Some(flag) = self.enabled.get(slot) {
            flag.store(enabled, Ordering::Relaxed);
        }
    }

    /// Set a parameter by index. Values are stored as relaxed atomics and read
    /// by the mixer on the next block.
    pub fn set_param(&self, slot: usize, param: usize, value: f32) {
        let value = if value.is_finite() { value } else { 0.0 };
        if let Some(slot) = self.params.get(slot)
            && let Some(cell) = slot.get(param)
        {
            cell.store(value.to_bits(), Ordering::Relaxed);
        }
    }

    /// Read the current bypass state of a slot.
    #[must_use]
    pub fn is_enabled(&self, slot: usize) -> bool {
        self.enabled
            .get(slot)
            .is_some_and(|flag| flag.load(Ordering::Relaxed))
    }

    /// Read the current value of a parameter.
    #[must_use]
    pub fn param(&self, slot: usize, param: usize) -> f32 {
        self.params
            .get(slot)
            .and_then(|row| row.get(param))
            .map(|cell| f32::from_bits(cell.load(Ordering::Relaxed)))
            .unwrap_or(0.0)
    }
}

/// Metadata for one insert slot, used by IPC and the effects UI.
#[derive(Debug, Clone, PartialEq)]
pub struct InsertSlotInfo {
    /// Zero-based slot position.
    pub index: usize,
    /// Stable effect id from the project.
    pub id: String,
    /// Effect kind from the project.
    pub kind: String,
    /// Ordered parameter names for this effect.
    pub param_names: Vec<String>,
    /// Whether the slot is currently active.
    pub enabled: bool,
}

/// A concrete DSP effect plus the parameter names it understands.
enum InsertEffect {
    Gain(Gain),
    Eq(ParametricEq),
    Compressor(Compressor),
    Limiter(Limiter),
    DeEsser(DeEsser),
    DeHum(DeHum),
    DeClick(DeClick),
    Gate(NoiseGate),
    Notch(SpectralNotchFilter),
    Harmonic(HarmonicSuppressor),
}

impl InsertEffect {
    fn name(&self) -> &'static str {
        match self {
            Self::Gain(effect) => effect.name(),
            Self::Eq(effect) => effect.name(),
            Self::Compressor(effect) => effect.name(),
            Self::Limiter(effect) => effect.name(),
            Self::DeEsser(effect) => effect.name(),
            Self::DeHum(effect) => effect.name(),
            Self::DeClick(effect) => effect.name(),
            Self::Gate(effect) => effect.name(),
            Self::Notch(effect) => effect.name(),
            Self::Harmonic(effect) => effect.name(),
        }
    }

    fn as_effect_mut(&mut self) -> &mut dyn Effect {
        match self {
            Self::Gain(effect) => effect,
            Self::Eq(effect) => effect,
            Self::Compressor(effect) => effect,
            Self::Limiter(effect) => effect,
            Self::DeEsser(effect) => effect,
            Self::DeHum(effect) => effect,
            Self::DeClick(effect) => effect,
            Self::Gate(effect) => effect,
            Self::Notch(effect) => effect,
            Self::Harmonic(effect) => effect,
        }
    }

    fn apply_param(&mut self, index: usize, value: f32) {
        match self {
            Self::Gain(effect) if index == 0 => *effect = Gain::new(value.max(0.0)).unwrap_or_else(|_| Gain::new(1.0).expect("unity gain")),
            Self::Eq(effect) if index < 3 => {
                let (frequency, q, gain) = match index {
                    0 => (value, 1.0, 0.0),
                    1 => (1_000.0, value, 0.0),
                    _ => (1_000.0, 1.0, value),
                };
                effect.set_band(0, frequency.max(10.0), q.max(0.1), gain);
            }
            Self::Notch(effect) => match index {
                0 => effect.set_center_hz(value),
                1 => effect.set_bandwidth_q(value),
                2 => effect.set_reduction_db(value),
                _ => {}
            },
            Self::Harmonic(effect) => match index {
                0 => effect.set_fundamental_hz(value),
                1 => effect.set_harmonics(value.round().max(0.0) as usize),
                2 => effect.set_bandwidth_q(value),
                3 => effect.set_reduction_db(value),
                _ => {}
            },
            // Legacy effects expose immutable configured state; their values are
            // retained in the control atomics and applied on the next rack build.
            _ => {}
        }
    }
}

struct InsertSlot {
    effect: InsertEffect,
    param_names: Vec<String>,
}

/// A prepared, fixed-size per-track insert chain.
pub struct InsertRack {
    slots: [Option<InsertSlot>; MAX_INSERT_SLOTS],
    control: InsertRackControl,
}

fn read(params: &BTreeMap<String, f64>, key: &str, default: f32) -> f32 {
    params
        .get(key)
        .copied()
        .map(|value| value as f32)
        .filter(|value| value.is_finite())
        .unwrap_or(default)
}

fn build_effect(effect: &EffectRef) -> (InsertEffect, Vec<(String, f32)>) {
    let params = &effect.params;
    let named = |list: &[(&str, f32)]| {
        list.iter()
            .map(|(name, default)| ((*name).to_owned(), *default))
            .collect::<Vec<_>>()
    };

    match effect.kind.as_str() {
        "gain" => (
            InsertEffect::Gain(Gain::new(read(params, "gain", 1.0).max(0.0)).unwrap_or_else(|_| Gain::new(1.0).expect("unity gain"))),
            named(&[("gain", 1.0)]),
        ),
        "eq" | "parametric_eq" => {
            let band = ParametricEq::with_bands(vec![EqBand::new(
                FilterType::Peaking,
                read(params, "band0_frequency_hz", 1_000.0).max(10.0),
                read(params, "band0_q", 1.0).max(0.1),
                read(params, "band0_gain_db", 0.0),
            )]);
            (
                InsertEffect::Eq(band),
                named(&[("band0_frequency_hz", 1_000.0), ("band0_q", 1.0), ("band0_gain_db", 0.0)]),
            )
        }
        "compressor" => (
            InsertEffect::Compressor(
                Compressor::new(
                    read(params, "threshold_db", -18.0),
                    read(params, "ratio", 3.0).max(1.0),
                    read(params, "attack_ms", 10.0).max(0.01),
                    read(params, "release_ms", 100.0).max(0.01),
                    read(params, "knee_db", 4.0).max(0.0),
                    read(params, "makeup_db", 2.0),
                )
                .unwrap_or_else(|_| Compressor::voice_default()),
            ),
            named(&[("threshold_db", -18.0), ("ratio", 3.0), ("attack_ms", 10.0), ("release_ms", 100.0), ("knee_db", 4.0), ("makeup_db", 2.0)]),
        ),
        "limiter" => (
            InsertEffect::Limiter(
                Limiter::new(
                    read(params, "ceiling_db", -1.0),
                    read(params, "attack_ms", 1.0).max(0.0),
                    read(params, "release_ms", 100.0).max(0.0),
                )
                .unwrap_or_else(|_| Limiter::new(-1.0, 1.0, 100.0).expect("valid limiter")),
            ),
            named(&[("ceiling_db", -1.0), ("attack_ms", 1.0), ("release_ms", 100.0)]),
        ),
        "de_esser" => (
            InsertEffect::DeEsser(
                DeEsser::new(
                    read(params, "frequency_hz", 6_000.0).max(1_000.0),
                    read(params, "threshold_db", -24.0),
                    read(params, "ratio", 3.0).max(1.0),
                )
                .unwrap_or_else(|_| DeEsser::new(6_000.0, -24.0, 3.0).expect("valid de-esser")),
            ),
            named(&[("frequency_hz", 6_000.0), ("threshold_db", -24.0), ("ratio", 3.0)]),
        ),
        "de_hum" => (
            InsertEffect::DeHum(DeHum::new(read(params, "mains_hz", 50.0))),
            named(&[("mains_hz", 50.0)]),
        ),
        "de_click" => (
            InsertEffect::DeClick(DeClick::new(read(params, "sensitivity", 0.5))),
            named(&[("sensitivity", 0.5)]),
        ),
        "noise_gate" => (
            InsertEffect::Gate(
                NoiseGate::new(
                    read(params, "threshold_db", -40.0),
                    read(params, "attack_ms", 5.0).max(0.01),
                    read(params, "hold_ms", 50.0).max(0.0),
                    read(params, "release_ms", 120.0).max(0.01),
                    read(params, "attenuation_db", 24.0).max(0.0),
                )
                .unwrap_or_else(|_| NoiseGate::voice_default()),
            ),
            named(&[("threshold_db", -40.0), ("attack_ms", 5.0), ("hold_ms", 50.0), ("release_ms", 120.0), ("attenuation_db", 24.0)]),
        ),
        "spectral_notch" => (
            InsertEffect::Notch(SpectralNotchFilter::new(
                read(params, "center_hz", 1_000.0).max(0.0),
                read(params, "bandwidth_q", 8.0).max(0.5),
                read(params, "reduction_db", 24.0).max(0.0),
            )),
            named(&[("center_hz", 1_000.0), ("bandwidth_q", 8.0), ("reduction_db", 24.0)]),
        ),
        "harmonic_suppressor" => (
            InsertEffect::Harmonic(HarmonicSuppressor::new(
                read(params, "fundamental_hz", 220.0).max(0.0),
                read(params, "harmonics", 4.0).max(0.0) as usize,
                read(params, "bandwidth_q", 8.0).max(0.5),
                read(params, "reduction_db", 24.0).max(0.0),
            )),
            named(&[("fundamental_hz", 220.0), ("harmonics", 4.0), ("bandwidth_q", 8.0), ("reduction_db", 24.0)]),
        ),
        _ => {
            tracing::warn!(kind = %effect.kind, "unknown insert effect; using unity gain");
            (
                InsertEffect::Gain(Gain::new(1.0).expect("unity gain")),
                named(&[("gain", 1.0)]),
            )
        }
    }
}

impl InsertRack {
    /// Build a rack from a track's effect chain.
    #[must_use]
    pub fn from_effects(effects: &[EffectRef], sample_rate: u32, channels: u16) -> Self {
        let control = InsertRackControl::default();
        let mut slots: [Option<InsertSlot>; MAX_INSERT_SLOTS] = std::array::from_fn(|_| None);
        for (index, effect) in effects.iter().take(MAX_INSERT_SLOTS).enumerate() {
            let (mut effect_impl, params) = build_effect(effect);
            effect_impl.as_effect_mut().prepare(sample_rate, channels);
            for (param_index, (name, default)) in params.iter().enumerate() {
                let value = read(&effect.params, name, *default);
                control.set_param(index, param_index, value);
            }
            control.set_enabled(index, effect.enabled);
            slots[index] = Some(InsertSlot {
                effect: effect_impl,
                param_names: params.into_iter().map(|(name, _)| name).collect(),
            });
        }
        Self { slots, control }
    }

    /// Build an empty rack.
    #[must_use]
    pub fn empty() -> Self {
        Self::from_effects(&[], 48_000, 2)
    }

    /// Clone the lock-free control handle for this rack.
    #[must_use]
    pub fn control(&self) -> InsertRackControl {
        self.control.clone()
    }

    /// Slot metadata for IPC and UI display.
    #[must_use]
    pub fn slot_info(&self, id_prefix: &str) -> Vec<InsertSlotInfo> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| {
                slot.as_ref().map(|slot| InsertSlotInfo {
                    index,
                    id: format!("{id_prefix}-insert-{index}"),
                    kind: slot.effect.name().to_owned(),
                    param_names: slot.param_names.clone(),
                    enabled: self.control.is_enabled(index),
                })
            })
            .collect()
    }

    /// Process all enabled inserts over an interleaved block.
    ///
    /// This runs on the audio thread: it reads relaxed atomics only, allocates
    /// nothing, and takes no locks.
    pub fn process(&mut self, buffer: &mut [f32]) {
        for (index, slot) in self.slots.iter_mut().enumerate() {
            if !self.control.is_enabled(index) {
                continue;
            }
            let Some(slot) = slot else { continue };
            for param in 0..slot.param_names.len().min(INSERT_PARAM_COUNT) {
                let value = self.control.param(index, param);
                slot.effect.apply_param(param, value);
            }
            slot.effect.as_effect_mut().process(buffer);
        }
    }

    /// Reset all insert state (used on transport seek).
    pub fn reset(&mut self) {
        for slot in self.slots.iter_mut().flatten() {
            slot.effect.as_effect_mut().reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn effect(kind: &str, enabled: bool, params: &[(&str, f64)]) -> EffectRef {
        let mut reference = EffectRef::new("test", kind);
        reference.enabled = enabled;
        for (key, value) in params {
            reference.params.insert((*key).to_owned(), *value);
        }
        reference
    }

    #[test]
    fn builds_up_to_eight_slots() {
        let effects: Vec<_> = (0..10)
            .map(|_| effect("gain", true, &[("gain", 0.5)]))
            .collect();
        let rack = InsertRack::from_effects(&effects, 48_000, 2);
        assert_eq!(rack.slot_info("track").len(), MAX_INSERT_SLOTS);
        assert_eq!(rack.control().param(0, 0), 0.5);
    }

    #[test]
    fn disabled_slot_is_bypassed() {
        let effects = [effect("gain", false, &[("gain", 0.0)])];
        let mut rack = InsertRack::from_effects(&effects, 48_000, 1);
        let mut buffer = [1.0; 16];
        rack.process(&mut buffer);
        assert_eq!(buffer, [1.0; 16]);
    }

    #[test]
    fn control_updates_are_visible_to_the_rack() {
        let effects = [effect("gain", true, &[("gain", 0.5)])];
        let mut rack = InsertRack::from_effects(&effects, 48_000, 1);
        let control = rack.control();
        control.set_param(0, 0, 0.25);
        let mut buffer = [1.0; 8];
        rack.process(&mut buffer);
        assert_eq!(buffer, [0.25; 8]);
    }

    #[test]
    fn unknown_kind_falls_back_to_unity_gain() {
        let effects = [effect("not-a-real-effect", true, &[])];
        let mut rack = InsertRack::from_effects(&effects, 48_000, 1);
        let mut buffer = [0.5; 4];
        rack.process(&mut buffer);
        assert_eq!(buffer, [0.5; 4]);
    }
}
