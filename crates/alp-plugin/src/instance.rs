//! Unified plugin instance interface for real-time audio block execution and state.

use std::collections::HashMap;

use crate::clap::ActiveClapInstance;
use crate::error::PluginError;
use crate::lv2::ActiveLv2Instance;
use crate::types::{PluginDescriptor, PluginFormat, PluginParameter, ProcessContext};

enum BackendInstance {
    Clap(ActiveClapInstance),
    Lv2(ActiveLv2Instance),
    Stub { gain: f32 },
}

/// A loaded, active audio plugin ready for real-time block processing.
pub struct ActivePlugin {
    descriptor: PluginDescriptor,
    sample_rate: u32,
    backend: BackendInstance,
    parameters: Vec<PluginParameter>,
    param_lookup: HashMap<String, usize>,
    bypassed: bool,
}

impl ActivePlugin {
    /// Instantiate a plugin from its descriptor.
    pub fn instantiate(desc: &PluginDescriptor, sample_rate: u32) -> Result<Self, PluginError> {
        let mut parameters = Vec::new();
        let mut param_lookup = HashMap::new();

        // Seed default parameters
        let default_params = [
            ("gain", "Gain", -60.0, 12.0, 0.0, "dB"),
            ("mix", "Dry / Wet Mix", 0.0, 100.0, 100.0, "%"),
            ("bypass", "Bypass", 0.0, 1.0, 0.0, "bool"),
        ];

        for (idx, (sym, name, min, max, def, unit)) in default_params.into_iter().enumerate() {
            parameters.push(PluginParameter {
                id: idx as u32,
                symbol: sym.to_string(),
                name: name.to_string(),
                min_value: min,
                max_value: max,
                default_value: def,
                current_value: def,
                unit: unit.to_string(),
            });
            param_lookup.insert(sym.to_string(), idx);
        }

        let backend = if desc.path.is_file() {
            match desc.format {
                PluginFormat::Clap => match ActiveClapInstance::load(desc, sample_rate) {
                    Ok(clap_inst) => BackendInstance::Clap(clap_inst),
                    Err(_) => BackendInstance::Stub { gain: 1.0 },
                },
                PluginFormat::Lv2 => match ActiveLv2Instance::load(desc, sample_rate) {
                    Ok(lv2_inst) => BackendInstance::Lv2(lv2_inst),
                    Err(_) => BackendInstance::Stub { gain: 1.0 },
                },
            }
        } else {
            BackendInstance::Stub { gain: 1.0 }
        };

        Ok(Self {
            descriptor: desc.clone(),
            sample_rate,
            backend,
            parameters,
            param_lookup,
            bypassed: false,
        })
    }

    /// Access the underlying plugin descriptor.
    #[must_use]
    pub fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    /// Whether the plugin is currently bypassed.
    #[must_use]
    pub fn is_bypassed(&self) -> bool {
        self.bypassed
    }

    /// Set bypass mode.
    pub fn set_bypassed(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    /// Sample rate.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Read all exposed parameters.
    #[must_use]
    pub fn parameters(&self) -> &[PluginParameter] {
        &self.parameters
    }

    /// Retrieve parameter value by symbol.
    #[must_use]
    pub fn get_parameter(&self, symbol: &str) -> Option<f64> {
        self.param_lookup
            .get(symbol)
            .and_then(|&idx| self.parameters.get(idx))
            .map(|p| p.current_value)
    }

    /// Set a parameter value by symbol.
    pub fn set_parameter(&mut self, symbol: &str, value: f64) -> Result<(), PluginError> {
        let idx = *self
            .param_lookup
            .get(symbol)
            .ok_or_else(|| PluginError::UnknownParameter {
                plugin_id: self.descriptor.id.clone(),
                param_id: 0,
            })?;

        let param = &mut self.parameters[idx];
        param.current_value = param.clamp(value);

        if symbol == "bypass" {
            self.bypassed = param.current_value > 0.5;
        }

        if let (BackendInstance::Stub { gain }, "gain") = (&mut self.backend, symbol) {
            *gain = 10.0f32.powf((param.current_value as f32) / 20.0);
        }

        Ok(())
    }

    /// Real-time audio block execution.
    pub fn process(
        &mut self,
        in_l: &[f32],
        in_r: &[f32],
        out_l: &mut [f32],
        out_r: &mut [f32],
        ctx: &ProcessContext,
    ) -> Result<(), PluginError> {
        let frames = in_l.len().min(in_r.len()).min(out_l.len()).min(out_r.len());
        if frames == 0 {
            return Ok(());
        }

        if self.bypassed {
            out_l[..frames].copy_from_slice(&in_l[..frames]);
            out_r[..frames].copy_from_slice(&in_r[..frames]);
            return Ok(());
        }

        match self.backend {
            BackendInstance::Clap(ref mut clap_inst) => {
                clap_inst.process_block(in_l, in_r, out_l, out_r, ctx)?;
            }
            BackendInstance::Lv2(ref mut lv2_inst) => {
                lv2_inst.process_block(in_l, in_r, out_l, out_r, ctx)?;
            }
            BackendInstance::Stub { gain } => {
                for i in 0..frames {
                    out_l[i] = in_l[i] * gain;
                    out_r[i] = in_r[i] * gain;
                }
            }
        }

        Ok(())
    }

    /// Export plugin parameters as a serialized state string.
    pub fn save_state(&self) -> Result<String, PluginError> {
        let map: HashMap<String, f64> = self
            .parameters
            .iter()
            .map(|p| (p.symbol.clone(), p.current_value))
            .collect();
        serde_json::to_string(&map).map_err(|e| PluginError::State {
            plugin_id: self.descriptor.id.clone(),
            message: e.to_string(),
        })
    }

    /// Restore plugin parameters from a serialized state string.
    pub fn restore_state(&mut self, state: &str) -> Result<(), PluginError> {
        let map: HashMap<String, f64> =
            serde_json::from_str(state).map_err(|e| PluginError::State {
                plugin_id: self.descriptor.id.clone(),
                message: e.to_string(),
            })?;

        for (sym, val) in map {
            let _ = self.set_parameter(&sym, val);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PluginCategory;
    use std::path::PathBuf;

    #[test]
    fn stub_instance_processes_audio_and_parameters() {
        let desc = PluginDescriptor {
            id: "stub.test".into(),
            name: "Stub Test Plugin".into(),
            vendor: "Audio Labs Pro".into(),
            version: "1.0.0".into(),
            format: PluginFormat::Clap,
            category: PluginCategory::Effect,
            path: PathBuf::from("/nonexistent/plugin.clap"),
            audio_inputs: 2,
            audio_outputs: 2,
            param_count: 3,
        };

        let mut instance = ActivePlugin::instantiate(&desc, 48000).expect("instantiate");
        assert_eq!(instance.parameters().len(), 3);

        let ctx = ProcessContext {
            sample_rate: 48000,
            num_frames: 4,
            frame_position: 0,
            is_playing: true,
        };

        let in_l = [1.0f32, 0.5, -0.5, -1.0];
        let in_r = [1.0f32, 0.5, -0.5, -1.0];
        let mut out_l = [0.0f32; 4];
        let mut out_r = [0.0f32; 4];

        instance
            .process(&in_l, &in_r, &mut out_l, &mut out_r, &ctx)
            .expect("process");
        assert_eq!(out_l, in_l);

        // Adjust gain to -6 dB (~0.5x)
        instance.set_parameter("gain", -6.0).expect("set gain");
        instance
            .process(&in_l, &in_r, &mut out_l, &mut out_r, &ctx)
            .expect("process");
        assert!((out_l[0] - 0.501).abs() < 0.01);

        // Save & restore state
        let state = instance.save_state().expect("save state");
        instance.set_parameter("gain", 0.0).expect("reset gain");
        instance.restore_state(&state).expect("restore");
        assert!((instance.get_parameter("gain").unwrap() - (-6.0)).abs() < 1e-4);
    }
}
