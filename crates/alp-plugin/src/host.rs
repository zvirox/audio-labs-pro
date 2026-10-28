//! High-level plugin host coordinating discovery, instances, and track integration.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::error::PluginError;
use crate::instance::ActivePlugin;
use crate::scanner::PluginScanner;
use crate::types::{PluginCategory, PluginDescriptor, PluginFormat};

/// Central audio plugin host managing discovery and loaded instances.
#[derive(Debug, Clone)]
pub struct PluginHost {
    scanner: PluginScanner,
    descriptors: Arc<Mutex<Vec<PluginDescriptor>>>,
}

impl Default for PluginHost {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginHost {
    /// Initialize a new host and seed built-in reference descriptors.
    #[must_use]
    pub fn new() -> Self {
        let scanner = PluginScanner::new();
        let mut descriptors = scanner.load_cache().unwrap_or_else(|| scanner.scan());

        // Always seed built-in virtual reference plugins for testing & offline render
        let virtual_plugins = [
            PluginDescriptor {
                id: "alp.vintage_tube".into(),
                name: "ALP Vintage Tube Saturation".into(),
                vendor: "Audio Labs Pro".into(),
                version: "1.2.0".into(),
                format: PluginFormat::Clap,
                category: PluginCategory::Effect,
                path: PathBuf::from("/usr/lib/clap/alp_vintage_tube.clap"),
                audio_inputs: 2,
                audio_outputs: 2,
                param_count: 5,
            },
            PluginDescriptor {
                id: "alp.master_multiband".into(),
                name: "ALP Master Multiband Dynamics".into(),
                vendor: "Audio Labs Pro".into(),
                version: "1.2.0".into(),
                format: PluginFormat::Lv2,
                category: PluginCategory::Effect,
                path: PathBuf::from("/usr/lib/lv2/alp_master_multiband.lv2"),
                audio_inputs: 2,
                audio_outputs: 2,
                param_count: 8,
            },
            PluginDescriptor {
                id: "alp.neural_denoiser".into(),
                name: "ALP Neural Voice Denoiser".into(),
                vendor: "Audio Labs Pro".into(),
                version: "1.2.0".into(),
                format: PluginFormat::Lv2,
                category: PluginCategory::Restoration,
                path: PathBuf::from("/usr/lib/lv2/alp_neural_denoiser.lv2"),
                audio_inputs: 2,
                audio_outputs: 2,
                param_count: 4,
            },
        ];

        for vp in virtual_plugins {
            if !descriptors.iter().any(|d| d.id == vp.id) {
                descriptors.push(vp);
            }
        }

        Self {
            scanner,
            descriptors: Arc::new(Mutex::new(descriptors)),
        }
    }

    /// Rescan standard paths for newly installed plugins.
    pub fn rescan(&self) -> Vec<PluginDescriptor> {
        let mut fresh = self.scanner.scan();
        let mut list = self.descriptors.lock().unwrap();
        // Preserve virtual plugins
        for d in list.iter() {
            if d.id.starts_with("alp.") && !fresh.iter().any(|f| f.id == d.id) {
                fresh.push(d.clone());
            }
        }
        *list = fresh.clone();
        fresh
    }

    /// List all discovered plugins.
    #[must_use]
    pub fn list_plugins(&self) -> Vec<PluginDescriptor> {
        self.descriptors.lock().unwrap().clone()
    }

    /// Find a plugin descriptor by unique ID.
    #[must_use]
    pub fn find_by_id(&self, id: &str) -> Option<PluginDescriptor> {
        self.descriptors
            .lock()
            .unwrap()
            .iter()
            .find(|d| d.id == id)
            .cloned()
    }

    /// Instantiate an active plugin by its unique ID.
    pub fn instantiate(&self, id: &str, sample_rate: u32) -> Result<ActivePlugin, PluginError> {
        let desc = self
            .find_by_id(id)
            .ok_or_else(|| PluginError::NotFound(id.to_string()))?;
        ActivePlugin::instantiate(&desc, sample_rate)
    }
}

/// Processing chain of active hosted plugins for one audio track or master bus.
pub struct TrackPluginChain {
    plugins: Vec<ActivePlugin>,
    scratch_in_l: Vec<f32>,
    scratch_in_r: Vec<f32>,
    scratch_out_l: Vec<f32>,
    scratch_out_r: Vec<f32>,
}

impl Default for TrackPluginChain {
    fn default() -> Self {
        Self::new()
    }
}

impl TrackPluginChain {
    /// Create an empty plugin chain with preallocated scratch buffers.
    #[must_use]
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            scratch_in_l: vec![0.0f32; 4096],
            scratch_in_r: vec![0.0f32; 4096],
            scratch_out_l: vec![0.0f32; 4096],
            scratch_out_r: vec![0.0f32; 4096],
        }
    }

    /// Number of active plugins in this chain.
    #[must_use]
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// Whether the chain is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    /// Insert a loaded plugin at the end of the chain.
    pub fn add_plugin(&mut self, plugin: ActivePlugin) {
        self.plugins.push(plugin);
    }

    /// Remove a plugin by index.
    pub fn remove_plugin(&mut self, index: usize) -> Option<ActivePlugin> {
        if index < self.plugins.len() {
            Some(self.plugins.remove(index))
        } else {
            None
        }
    }

    /// Access loaded plugins.
    #[must_use]
    pub fn plugins(&self) -> &[ActivePlugin] {
        &self.plugins
    }

    /// Mutable access to loaded plugins.
    pub fn plugins_mut(&mut self) -> &mut [ActivePlugin] {
        &mut self.plugins
    }

    fn ensure_capacity(&mut self, frames: usize) {
        if self.scratch_in_l.len() < frames {
            self.scratch_in_l.resize(frames, 0.0);
            self.scratch_in_r.resize(frames, 0.0);
            self.scratch_out_l.resize(frames, 0.0);
            self.scratch_out_r.resize(frames, 0.0);
        }
    }

    /// Real-time serial execution of all active plugins in the chain.
    pub fn process_chain(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        ctx: &crate::types::ProcessContext,
    ) -> Result<(), PluginError> {
        let frames = left.len().min(right.len());
        if frames == 0 || self.plugins.is_empty() {
            return Ok(());
        }
        self.ensure_capacity(frames);

        for plugin in &mut self.plugins {
            if plugin.is_bypassed() {
                continue;
            }
            plugin.process(
                left,
                right,
                &mut self.scratch_out_l[..frames],
                &mut self.scratch_out_r[..frames],
                ctx,
            )?;
            left[..frames].copy_from_slice(&self.scratch_out_l[..frames]);
            right[..frames].copy_from_slice(&self.scratch_out_r[..frames]);
        }

        Ok(())
    }

    /// Process interleaved audio buffer in-place using preallocated scratch buffers.
    pub fn process_interleaved(
        &mut self,
        buffer: &mut [f32],
        channels: usize,
        ctx: &crate::types::ProcessContext,
    ) -> Result<(), PluginError> {
        if self.plugins.is_empty() || channels == 0 {
            return Ok(());
        }
        let frames = buffer.len() / channels;
        if frames == 0 {
            return Ok(());
        }
        self.ensure_capacity(frames);

        if channels == 1 {
            for (i, sample) in buffer.iter().enumerate() {
                self.scratch_in_l[i] = *sample;
                self.scratch_in_r[i] = *sample;
            }
        } else {
            for i in 0..frames {
                self.scratch_in_l[i] = buffer[i * channels];
                self.scratch_in_r[i] = buffer[i * channels + 1];
            }
        }

        for plugin in &mut self.plugins {
            if plugin.is_bypassed() {
                continue;
            }
            plugin.process(
                &self.scratch_in_l[..frames],
                &self.scratch_in_r[..frames],
                &mut self.scratch_out_l[..frames],
                &mut self.scratch_out_r[..frames],
                ctx,
            )?;
            self.scratch_in_l[..frames].copy_from_slice(&self.scratch_out_l[..frames]);
            self.scratch_in_r[..frames].copy_from_slice(&self.scratch_out_r[..frames]);
        }

        if channels == 1 {
            for (i, sample) in buffer.iter_mut().enumerate().take(frames) {
                *sample = (self.scratch_in_l[i] + self.scratch_in_r[i]) * 0.5;
            }
        } else {
            for i in 0..frames {
                buffer[i * channels] = self.scratch_in_l[i];
                buffer[i * channels + 1] = self.scratch_in_r[i];
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_initialization_lists_and_instantiates_plugins() {
        let host = PluginHost::new();
        let list = host.list_plugins();
        assert!(!list.is_empty());

        let plugin = host
            .instantiate("alp.vintage_tube", 48000)
            .expect("instantiate virtual tube");
        assert_eq!(plugin.descriptor().name, "ALP Vintage Tube Saturation");
    }

    #[test]
    fn track_plugin_chain_processes_in_series() {
        let host = PluginHost::new();
        let mut chain = TrackPluginChain::new();

        let mut p1 = host.instantiate("alp.vintage_tube", 48000).unwrap();
        let mut p2 = host.instantiate("alp.master_multiband", 48000).unwrap();

        p1.set_parameter("gain", -6.0).unwrap();
        p2.set_parameter("gain", 6.0).unwrap();

        chain.add_plugin(p1);
        chain.add_plugin(p2);
        assert_eq!(chain.len(), 2);

        let mut left = [1.0f32; 8];
        let mut right = [1.0f32; 8];
        let ctx = crate::types::ProcessContext {
            sample_rate: 48000,
            num_frames: 8,
            frame_position: 0,
            is_playing: true,
        };

        chain
            .process_chain(&mut left, &mut right, &ctx)
            .expect("process chain");
        assert!((left[0] - 1.0).abs() < 0.05);
    }

    #[test]
    fn track_plugin_chain_processes_interleaved() {
        let host = PluginHost::new();
        let mut chain = TrackPluginChain::new();
        let mut p1 = host.instantiate("alp.vintage_tube", 48000).unwrap();
        p1.set_parameter("gain", -6.0).unwrap();
        chain.add_plugin(p1);

        let mut interleaved = [1.0f32; 16]; // 8 frames stereo
        let ctx = crate::types::ProcessContext {
            sample_rate: 48000,
            num_frames: 8,
            frame_position: 0,
            is_playing: true,
        };

        chain
            .process_interleaved(&mut interleaved, 2, &ctx)
            .expect("process interleaved");
        assert!(interleaved[0] < 0.6);
        assert_eq!(interleaved[0], interleaved[1]);
    }
}
