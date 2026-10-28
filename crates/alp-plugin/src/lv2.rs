//! Safe Rust host interface and C ABI definitions for LV2 (Linux Audio Plugin Standard).

use std::ffi::{CString, c_char, c_void};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use libloading::{Library, Symbol};

use crate::error::PluginError;
use crate::types::{PluginCategory, PluginDescriptor, PluginFormat, ProcessContext};

/// Opaque pointer representing an active LV2 plugin instance.
pub type Lv2Handle = *mut c_void;

/// Feature pointer passed to LV2 instantiate.
#[repr(C)]
pub struct Lv2Feature {
    /// URI of the feature.
    pub uri: *const c_char,
    /// Feature-specific data.
    pub data: *mut c_void,
}

/// LV2 Plugin descriptor matching the LV2 C ABI (`lv2.h`).
#[repr(C)]
pub struct Lv2Descriptor {
    /// Unique URI identifying this plugin.
    pub uri: *const c_char,
    /// Instantiate a plugin.
    pub instantiate: Option<
        unsafe extern "C" fn(
            descriptor: *const Lv2Descriptor,
            sample_rate: f64,
            bundle_path: *const c_char,
            features: *const *const Lv2Feature,
        ) -> Lv2Handle,
    >,
    /// Connect a port index to a memory buffer.
    pub connect_port:
        Option<unsafe extern "C" fn(instance: Lv2Handle, port: u32, data_location: *mut c_void)>,
    /// Activate processing.
    pub activate: Option<unsafe extern "C" fn(instance: Lv2Handle)>,
    /// Real-time audio processing block callback.
    pub run: Option<unsafe extern "C" fn(instance: Lv2Handle, sample_count: u32)>,
    /// Deactivate processing.
    pub deactivate: Option<unsafe extern "C" fn(instance: Lv2Handle)>,
    /// Free the plugin instance.
    pub cleanup: Option<unsafe extern "C" fn(instance: Lv2Handle)>,
    /// Extension data callback.
    pub extension_data: Option<unsafe extern "C" fn(uri: *const c_char) -> *const c_void>,
}

/// Parse an LV2 bundle directory (`.lv2`) reading `manifest.ttl`.
pub fn scan_lv2_bundle(bundle_dir: &Path) -> Result<Vec<PluginDescriptor>, PluginError> {
    let manifest_path = bundle_dir.join("manifest.ttl");
    if !manifest_path.is_file() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&manifest_path).map_err(|e| PluginError::Discovery {
        path: manifest_path.clone(),
        message: e.to_string(),
    })?;

    let mut descriptors = Vec::new();
    let mut current_binary: Option<PathBuf> = None;
    let mut current_uri: Option<String> = None;

    // Scan for binary .so links in the bundle directory
    if let Ok(entries) = fs::read_dir(bundle_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().is_some_and(|ext| ext == "so") {
                current_binary = Some(p);
                break;
            }
        }
    }

    // Parse simple Turtle manifest statements
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('<') && trimmed.contains("lv2:binary") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 3 {
                let bin_rel =
                    parts[2].trim_matches(|c| c == '<' || c == '>' || c == ';' || c == '.');
                let bin_path = bundle_dir.join(bin_rel);
                if bin_path.exists() {
                    current_binary = Some(bin_path);
                }
            }
        } else if trimmed.starts_with('<') && trimmed.contains("a lv2:Plugin") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if let Some(first) = parts.first() {
                current_uri = Some(first.trim_matches(|c| c == '<' || c == '>').to_string());
            }
        }
    }

    let bundle_name = bundle_dir
        .file_stem()
        .map_or("Unknown LV2", |s| s.to_str().unwrap_or("LV2 Plugin"));

    if let Some(binary_path) = current_binary {
        let uri = current_uri
            .unwrap_or_else(|| format!("http://audiolabspro.local/plugins/lv2/{}", bundle_name));

        descriptors.push(PluginDescriptor {
            id: uri,
            name: bundle_name.replace('_', " ").replace(".lv2", ""),
            vendor: "Linux Audio (LV2)".into(),
            version: "1.0.0".into(),
            format: PluginFormat::Lv2,
            category: if bundle_name.contains("noise") {
                PluginCategory::Restoration
            } else {
                PluginCategory::Effect
            },
            path: binary_path,
            audio_inputs: 2,
            audio_outputs: 2,
            param_count: 10,
        });
    }

    Ok(descriptors)
}

/// An active, instantiated LV2 plugin instance.
pub struct ActiveLv2Instance {
    _library: Arc<Library>,
    descriptor_ptr: *const Lv2Descriptor,
    handle: Lv2Handle,
    descriptor: PluginDescriptor,
    active: bool,
    in_buf_left: Vec<f32>,
    in_buf_right: Vec<f32>,
    out_buf_left: Vec<f32>,
    out_buf_right: Vec<f32>,
}

unsafe impl Send for ActiveLv2Instance {}

impl ActiveLv2Instance {
    /// Instantiate an LV2 plugin from its binary descriptor.
    #[allow(unsafe_code)]
    pub fn load(desc: &PluginDescriptor, sample_rate: u32) -> Result<Self, PluginError> {
        let lib = Arc::new(unsafe {
            Library::new(&desc.path).map_err(|e| PluginError::LibraryLoad {
                path: desc.path.clone(),
                message: e.to_string(),
            })?
        });

        let (descriptor_ptr, handle) = unsafe {
            let get_descriptor: Symbol<unsafe extern "C" fn(u32) -> *const Lv2Descriptor> = lib
                .get(b"lv2_descriptor")
                .map_err(|e| PluginError::MissingSymbol {
                    path: desc.path.clone(),
                    symbol: format!("lv2_descriptor: {e}"),
                })?;

            let desc_ptr = get_descriptor(0);
            if desc_ptr.is_null() {
                return Err(PluginError::Instantiation {
                    plugin_id: desc.id.clone(),
                    message: "lv2_descriptor(0) returned null".into(),
                });
            }

            let descriptor = &*desc_ptr;
            let instantiate_fn =
                descriptor
                    .instantiate
                    .ok_or_else(|| PluginError::Instantiation {
                        plugin_id: desc.id.clone(),
                        message: "descriptor.instantiate is null".into(),
                    })?;

            let parent_dir = desc.path.parent().unwrap_or(&desc.path);
            let c_bundle = CString::new(parent_dir.to_string_lossy().as_bytes()).map_err(|e| {
                PluginError::Instantiation {
                    plugin_id: desc.id.clone(),
                    message: e.to_string(),
                }
            })?;

            let features_null: [*const Lv2Feature; 1] = [std::ptr::null()];
            let instance_handle = instantiate_fn(
                desc_ptr,
                sample_rate as f64,
                c_bundle.as_ptr(),
                features_null.as_ptr(),
            );

            if instance_handle.is_null() {
                return Err(PluginError::Instantiation {
                    plugin_id: desc.id.clone(),
                    message: "instantiate returned null handle".into(),
                });
            }

            (desc_ptr, instance_handle)
        };

        let mut instance = Self {
            _library: lib,
            descriptor_ptr,
            handle,
            descriptor: desc.clone(),
            active: false,
            in_buf_left: vec![0.0f32; 4096],
            in_buf_right: vec![0.0f32; 4096],
            out_buf_left: vec![0.0f32; 4096],
            out_buf_right: vec![0.0f32; 4096],
        };

        instance.activate()?;
        Ok(instance)
    }

    /// Get reference to plugin descriptor.
    pub fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    /// Activate LV2 processing.
    #[allow(unsafe_code)]
    pub fn activate(&mut self) -> Result<(), PluginError> {
        if self.active {
            return Ok(());
        }
        unsafe {
            let descriptor = &*self.descriptor_ptr;
            if let Some(act) = descriptor.activate {
                act(self.handle);
            }
        }
        self.active = true;
        Ok(())
    }

    /// Deactivate LV2 processing.
    #[allow(unsafe_code)]
    pub fn deactivate(&mut self) {
        if !self.active {
            return;
        }
        unsafe {
            let descriptor = &*self.descriptor_ptr;
            if let Some(deact) = descriptor.deactivate {
                deact(self.handle);
            }
        }
        self.active = false;
    }

    /// Process a stereo block of audio frames in real time.
    #[allow(unsafe_code)]
    pub fn process_block(
        &mut self,
        in_l: &[f32],
        in_r: &[f32],
        out_l: &mut [f32],
        out_r: &mut [f32],
        _ctx: &ProcessContext,
    ) -> Result<(), PluginError> {
        let frames = in_l.len().min(in_r.len()).min(out_l.len()).min(out_r.len());
        if frames == 0 {
            return Ok(());
        }

        self.in_buf_left[..frames].copy_from_slice(&in_l[..frames]);
        self.in_buf_right[..frames].copy_from_slice(&in_r[..frames]);

        unsafe {
            let descriptor = &*self.descriptor_ptr;
            if let Some(connect) = descriptor.connect_port {
                // Connect stereo audio ports (port 0 = in_l, port 1 = in_r, port 2 = out_l, port 3 = out_r)
                connect(self.handle, 0, self.in_buf_left.as_mut_ptr() as *mut c_void);
                connect(
                    self.handle,
                    1,
                    self.in_buf_right.as_mut_ptr() as *mut c_void,
                );
                connect(
                    self.handle,
                    2,
                    self.out_buf_left.as_mut_ptr() as *mut c_void,
                );
                connect(
                    self.handle,
                    3,
                    self.out_buf_right.as_mut_ptr() as *mut c_void,
                );
            }

            if let Some(run_fn) = descriptor.run {
                run_fn(self.handle, frames as u32);
            }
        }

        out_l[..frames].copy_from_slice(&self.out_buf_left[..frames]);
        out_r[..frames].copy_from_slice(&self.out_buf_right[..frames]);
        Ok(())
    }
}

impl Drop for ActiveLv2Instance {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        self.deactivate();
        unsafe {
            let descriptor = &*self.descriptor_ptr;
            if let Some(cleanup_fn) = descriptor.cleanup {
                cleanup_fn(self.handle);
            }
        }
    }
}
