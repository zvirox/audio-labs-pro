//! Shared types, descriptors, parameter definitions, and plugin formats.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Supported open audio plugin standard formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginFormat {
    /// CLAP (CLever Audio Plugin) standard.
    Clap,
    /// LV2 Linux Audio standard.
    Lv2,
}

impl PluginFormat {
    /// Display name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Clap => "CLAP",
            Self::Lv2 => "LV2",
        }
    }

    /// Primary file extension or bundle directory extension.
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Self::Clap => "clap",
            Self::Lv2 => "lv2",
        }
    }
}

/// High-level functional category of a plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PluginCategory {
    /// Audio effect (EQ, compression, reverb, distortion, delay).
    #[default]
    Effect,
    /// Instrument or synthesizer producing audio from events.
    Instrument,
    /// Audio restoration, noise reduction, de-clicking, de-humming.
    Restoration,
    /// Audio analysis, metering, visualization.
    Analyzer,
    /// Routing, gain, channel utility.
    Utility,
}

/// Metadata and operational capabilities discovered for a plugin.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginDescriptor {
    /// Unique stable identifier (e.g. URI or reverse-DNS string).
    pub id: String,
    /// Human-readable plugin name.
    pub name: String,
    /// Author or developer vendor name.
    pub vendor: String,
    /// Version string.
    pub version: String,
    /// Underlying plugin binary format.
    pub format: PluginFormat,
    /// Functional category.
    pub category: PluginCategory,
    /// Filesystem location of the binary or bundle.
    pub path: PathBuf,
    /// Number of audio input channels supported.
    pub audio_inputs: usize,
    /// Number of audio output channels supported.
    pub audio_outputs: usize,
    /// Total count of exposed automatable parameters.
    pub param_count: usize,
}

/// Parameter definition for real-time automation and UI rendering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginParameter {
    /// Parameter numeric identifier.
    pub id: u32,
    /// Stable parameter symbol / key.
    pub symbol: String,
    /// Human-readable label.
    pub name: String,
    /// Minimum allowed value.
    pub min_value: f64,
    /// Maximum allowed value.
    pub max_value: f64,
    /// Default factory value.
    pub default_value: f64,
    /// Current value.
    pub current_value: f64,
    /// Unit label (e.g. "dB", "Hz", "%", "ms").
    pub unit: String,
}

impl PluginParameter {
    /// Clamp a value within parameter boundaries.
    #[must_use]
    pub fn clamp(&self, val: f64) -> f64 {
        val.clamp(self.min_value, self.max_value)
    }

    /// Normalized fraction in 0.0..=1.0.
    #[must_use]
    pub fn normalized(&self) -> f64 {
        let range = self.max_value - self.min_value;
        if range.abs() < 1e-9 {
            0.0
        } else {
            ((self.current_value - self.min_value) / range).clamp(0.0, 1.0)
        }
    }

    /// Set value from a normalized fraction in 0.0..=1.0.
    pub fn set_from_normalized(&mut self, norm: f64) {
        let clamped_norm = norm.clamp(0.0, 1.0);
        self.current_value = self.min_value + clamped_norm * (self.max_value - self.min_value);
    }
}

/// Execution context passed to real-time audio block processing.
#[derive(Debug, Clone, Copy)]
pub struct ProcessContext {
    /// Audio sample rate in Hz.
    pub sample_rate: u32,
    /// Number of audio frames in the current block.
    pub num_frames: usize,
    /// Steady playhead timestamp in audio frames.
    pub frame_position: u64,
    /// Whether the host transport is currently playing.
    pub is_playing: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameter_normalization_and_clamping() {
        let mut param = PluginParameter {
            id: 1,
            symbol: "gain".into(),
            name: "Master Gain".into(),
            min_value: -60.0,
            max_value: 12.0,
            default_value: 0.0,
            current_value: 0.0,
            unit: "dB".into(),
        };

        let norm = param.normalized();
        assert!((norm - (60.0 / 72.0)).abs() < 1e-4);

        param.set_from_normalized(0.0);
        assert_eq!(param.current_value, -60.0);

        param.set_from_normalized(1.0);
        assert_eq!(param.current_value, 12.0);
    }
}
