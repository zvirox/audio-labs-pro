//! Safe Rust host interface and C ABI definitions for CLAP (CLever Audio Plugin).

use std::ffi::{CStr, CString, c_char, c_void};
use std::path::Path;
use std::sync::Arc;

use libloading::{Library, Symbol};

use crate::error::PluginError;
use crate::types::{PluginCategory, PluginDescriptor, PluginFormat, ProcessContext};

/// CLAP specification version tuple.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClapVersion {
    /// Major revision.
    pub major: u32,
    /// Minor revision.
    pub minor: u32,
    /// Patch/revision.
    pub revision: u32,
}

/// Standard CLAP version supported by Audio Labs Pro (1.2.0).
pub const CLAP_VERSION: ClapVersion = ClapVersion {
    major: 1,
    minor: 2,
    revision: 0,
};

/// Plugin descriptor matching the CLAP C ABI.
#[repr(C)]
pub struct ClapPluginDescriptor {
    /// Supported CLAP version.
    pub clap_version: ClapVersion,
    /// Plugin unique identifier.
    pub id: *const c_char,
    /// Display name.
    pub name: *const c_char,
    /// Vendor string.
    pub vendor: *const c_char,
    /// Project URL.
    pub url: *const c_char,
    /// Manual URL.
    pub manual_url: *const c_char,
    /// Support URL.
    pub support_url: *const c_char,
    /// Version string.
    pub version: *const c_char,
    /// Short description.
    pub description: *const c_char,
    /// Null-terminated array of feature tags.
    pub features: *const *const c_char,
}

/// Host callback descriptor passed to the plugin.
#[repr(C)]
pub struct ClapHost {
    /// Host CLAP version.
    pub clap_version: ClapVersion,
    /// Host opaque pointer.
    pub host_data: *mut c_void,
    /// Host name.
    pub name: *const c_char,
    /// Host vendor.
    pub vendor: *const c_char,
    /// Host URL.
    pub url: *const c_char,
    /// Host version.
    pub version: *const c_char,
    /// Request host extension.
    pub get_extension: Option<
        unsafe extern "C" fn(host: *const ClapHost, extension_id: *const c_char) -> *const c_void,
    >,
    /// Request processing restart.
    pub request_restart: Option<unsafe extern "C" fn(host: *const ClapHost)>,
    /// Request processing cycle.
    pub request_process: Option<unsafe extern "C" fn(host: *const ClapHost)>,
    /// Request callback on main thread.
    pub request_callback: Option<unsafe extern "C" fn(host: *const ClapHost)>,
}

// Safety: ClapHost is static and read-only during plugin execution.
unsafe impl Sync for ClapHost {}
unsafe impl Send for ClapHost {}

/// Audio buffer for one bus matching CLAP C ABI.
#[repr(C)]
pub struct ClapAudioBuffer {
    /// Array of float channel sample pointers.
    pub data32: *mut *mut f32,
    /// Array of double channel sample pointers (or null).
    pub data64: *mut *mut f64,
    /// Channel count.
    pub channel_count: u32,
    /// Buffer latency frames.
    pub latency: u32,
    /// Constant mask (channels that are constant).
    pub constant_mask: u64,
}

/// Process context matching the CLAP C ABI.
#[repr(C)]
pub struct ClapProcess {
    /// Steady playhead sample time.
    pub steady_time: i64,
    /// Frame count in this processing block.
    pub frames_count: u32,
    /// Transport info pointer.
    pub transport: *const c_void,
    /// Input audio buffers array.
    pub audio_inputs: *const ClapAudioBuffer,
    /// Output audio buffers array.
    pub audio_outputs: *mut ClapAudioBuffer,
    /// Input bus count.
    pub audio_inputs_count: u32,
    /// Output bus count.
    pub audio_outputs_count: u32,
    /// Input event sequence.
    pub in_events: *const c_void,
    /// Output event sequence.
    pub out_events: *mut c_void,
}

/// Plugin instance methods matching CLAP C ABI.
#[repr(C)]
pub struct ClapPlugin {
    /// Descriptor pointer.
    pub desc: *const ClapPluginDescriptor,
    /// Private plugin instance data.
    pub plugin_data: *mut c_void,
    /// Initialization.
    pub init: Option<unsafe extern "C" fn(plugin: *const ClapPlugin) -> bool>,
    /// Destruction.
    pub destroy: Option<unsafe extern "C" fn(plugin: *const ClapPlugin)>,
    /// Activation with sample rate and block sizes.
    pub activate: Option<
        unsafe extern "C" fn(
            plugin: *const ClapPlugin,
            sample_rate: f64,
            min_frames: u32,
            max_frames: u32,
        ) -> bool,
    >,
    /// Deactivation.
    pub deactivate: Option<unsafe extern "C" fn(plugin: *const ClapPlugin)>,
    /// Start processing.
    pub start_processing: Option<unsafe extern "C" fn(plugin: *const ClapPlugin) -> bool>,
    /// Stop processing.
    pub stop_processing: Option<unsafe extern "C" fn(plugin: *const ClapPlugin)>,
    /// Reset DSP state.
    pub reset: Option<unsafe extern "C" fn(plugin: *const ClapPlugin)>,
    /// Process audio block.
    pub process:
        Option<unsafe extern "C" fn(plugin: *const ClapPlugin, process: *const ClapProcess) -> i32>,
    /// Query plugin extension.
    pub get_extension:
        Option<unsafe extern "C" fn(plugin: *const ClapPlugin, id: *const c_char) -> *const c_void>,
    /// Main thread tick.
    pub on_main_thread: Option<unsafe extern "C" fn(plugin: *const ClapPlugin)>,
}

/// Plugin entry struct loaded from `clap_entry`.
#[repr(C)]
pub struct ClapPluginEntry {
    /// CLAP version.
    pub clap_version: ClapVersion,
    /// Initialization.
    pub init: Option<unsafe extern "C" fn(plugin_path: *const c_char) -> bool>,
    /// Deinitialization.
    pub deinit: Option<unsafe extern "C" fn()>,
    /// Query factory.
    pub get_factory: Option<unsafe extern "C" fn(factory_id: *const c_char) -> *const c_void>,
}

/// Plugin factory struct.
#[repr(C)]
pub struct ClapPluginFactory {
    /// Plugin count.
    pub get_plugin_count: Option<unsafe extern "C" fn(factory: *const c_void) -> u32>,
    /// Plugin descriptor by index.
    pub get_plugin_descriptor: Option<
        unsafe extern "C" fn(factory: *const c_void, index: u32) -> *const ClapPluginDescriptor,
    >,
    /// Instantiate a plugin by ID.
    pub create_plugin: Option<
        unsafe extern "C" fn(
            factory: *const c_void,
            host: *const ClapHost,
            plugin_id: *const c_char,
        ) -> *const ClapPlugin,
    >,
}

static HOST_NAME: &[u8] = b"Audio Labs Pro\0";
static HOST_VENDOR: &[u8] = b"Antigravity\0";
static HOST_URL: &[u8] = b"https://github.com/zvirox/audio-labs-pro\0";
static HOST_VERSION: &[u8] = b"1.2.0\0";

static GLOBAL_CLAP_HOST: ClapHost = ClapHost {
    clap_version: CLAP_VERSION,
    host_data: std::ptr::null_mut(),
    name: HOST_NAME.as_ptr() as *const c_char,
    vendor: HOST_VENDOR.as_ptr() as *const c_char,
    url: HOST_URL.as_ptr() as *const c_char,
    version: HOST_VERSION.as_ptr() as *const c_char,
    get_extension: None,
    request_restart: None,
    request_process: None,
    request_callback: None,
};

/// Scan a single `.clap` shared library file for plugin descriptors.
#[allow(unsafe_code)]
pub fn scan_clap_file(path: &Path) -> Result<Vec<PluginDescriptor>, PluginError> {
    let lib = unsafe {
        Library::new(path).map_err(|e| PluginError::LibraryLoad {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?
    };

    let entry: Symbol<*const ClapPluginEntry> = unsafe {
        lib.get(b"clap_entry")
            .map_err(|e| PluginError::MissingSymbol {
                path: path.to_path_buf(),
                symbol: format!("clap_entry: {e}"),
            })?
    };

    if entry.is_null() {
        return Err(PluginError::MissingSymbol {
            path: path.to_path_buf(),
            symbol: "clap_entry is null".into(),
        });
    }

    let mut descriptors = Vec::new();
    unsafe {
        let entry_ref = &**entry;
        let c_path = CString::new(path.to_string_lossy().as_bytes()).map_err(|e| {
            PluginError::Discovery {
                path: path.to_path_buf(),
                message: e.to_string(),
            }
        })?;

        if entry_ref
            .init
            .is_some_and(|init_fn| !init_fn(c_path.as_ptr()))
        {
            return Err(PluginError::Instantiation {
                plugin_id: path.display().to_string(),
                message: "clap_entry init returned false".into(),
            });
        }

        let factory_id =
            CString::new("clap.plugin-factory").map_err(|e| PluginError::Discovery {
                path: path.to_path_buf(),
                message: e.to_string(),
            })?;
        if let Some(get_factory) = entry_ref.get_factory {
            let factory_ptr = get_factory(factory_id.as_ptr()) as *const ClapPluginFactory;
            if !factory_ptr.is_null() {
                let factory = &*factory_ptr;
                let count = factory
                    .get_plugin_count
                    .map_or(0, |f| f(factory_ptr as *const c_void));
                for i in 0..count {
                    if let Some(get_desc) = factory.get_plugin_descriptor {
                        let desc_ptr = get_desc(factory_ptr as *const c_void, i);
                        if !desc_ptr.is_null() {
                            let desc = &*desc_ptr;
                            let id = CStr::from_ptr(desc.id).to_string_lossy().into_owned();
                            let name = CStr::from_ptr(desc.name).to_string_lossy().into_owned();
                            let vendor = CStr::from_ptr(desc.vendor).to_string_lossy().into_owned();
                            let version =
                                CStr::from_ptr(desc.version).to_string_lossy().into_owned();

                            descriptors.push(PluginDescriptor {
                                id,
                                name,
                                vendor,
                                version,
                                format: PluginFormat::Clap,
                                category: PluginCategory::Effect,
                                path: path.to_path_buf(),
                                audio_inputs: 2,
                                audio_outputs: 2,
                                param_count: 8,
                            });
                        }
                    }
                }
            }
        }

        if let Some(deinit_fn) = entry_ref.deinit {
            deinit_fn();
        }
    }

    Ok(descriptors)
}

/// An active, loaded CLAP plugin instance.
pub struct ActiveClapInstance {
    _library: Arc<Library>,
    plugin_ptr: *const ClapPlugin,
    descriptor: PluginDescriptor,
    sample_rate: u32,
    active: bool,
    in_buf_left: Vec<f32>,
    in_buf_right: Vec<f32>,
    out_buf_left: Vec<f32>,
    out_buf_right: Vec<f32>,
}

unsafe impl Send for ActiveClapInstance {}

impl ActiveClapInstance {
    /// Load and instantiate a CLAP plugin.
    #[allow(unsafe_code)]
    pub fn load(desc: &PluginDescriptor, sample_rate: u32) -> Result<Self, PluginError> {
        let lib = Arc::new(unsafe {
            Library::new(&desc.path).map_err(|e| PluginError::LibraryLoad {
                path: desc.path.clone(),
                message: e.to_string(),
            })?
        });

        let plugin_ptr = unsafe {
            let entry: Symbol<*const ClapPluginEntry> =
                lib.get(b"clap_entry")
                    .map_err(|e| PluginError::MissingSymbol {
                        path: desc.path.clone(),
                        symbol: format!("clap_entry: {e}"),
                    })?;

            if entry.is_null() {
                return Err(PluginError::MissingSymbol {
                    path: desc.path.clone(),
                    symbol: "clap_entry is null".into(),
                });
            }

            let entry_ref = &**entry;
            let c_path = CString::new(desc.path.to_string_lossy().as_bytes()).map_err(|e| {
                PluginError::Instantiation {
                    plugin_id: desc.id.clone(),
                    message: e.to_string(),
                }
            })?;

            if let Some(init_fn) = entry_ref.init {
                let _ = init_fn(c_path.as_ptr());
            }

            let factory_id = CString::new("clap.plugin-factory").unwrap();
            let get_factory = entry_ref
                .get_factory
                .ok_or_else(|| PluginError::Instantiation {
                    plugin_id: desc.id.clone(),
                    message: "missing get_factory".into(),
                })?;

            let factory_ptr = get_factory(factory_id.as_ptr()) as *const ClapPluginFactory;
            if factory_ptr.is_null() {
                return Err(PluginError::Instantiation {
                    plugin_id: desc.id.clone(),
                    message: "plugin factory is null".into(),
                });
            }

            let factory = &*factory_ptr;
            let create_plugin =
                factory
                    .create_plugin
                    .ok_or_else(|| PluginError::Instantiation {
                        plugin_id: desc.id.clone(),
                        message: "missing create_plugin".into(),
                    })?;

            let plugin_c_id = CString::new(desc.id.as_bytes()).unwrap();
            let inst = create_plugin(
                factory_ptr as *const c_void,
                &GLOBAL_CLAP_HOST,
                plugin_c_id.as_ptr(),
            );
            if inst.is_null() {
                return Err(PluginError::Instantiation {
                    plugin_id: desc.id.clone(),
                    message: "create_plugin returned null".into(),
                });
            }

            let plugin = &*inst;
            if plugin.init.is_some_and(|init_inst| !init_inst(inst)) {
                return Err(PluginError::Instantiation {
                    plugin_id: desc.id.clone(),
                    message: "plugin.init returned false".into(),
                });
            }

            inst
        };

        let mut instance = Self {
            _library: lib,
            plugin_ptr,
            descriptor: desc.clone(),
            sample_rate,
            active: false,
            in_buf_left: vec![0.0f32; 4096],
            in_buf_right: vec![0.0f32; 4096],
            out_buf_left: vec![0.0f32; 4096],
            out_buf_right: vec![0.0f32; 4096],
        };

        instance.activate()?;
        Ok(instance)
    }

    /// Activate DSP processing.
    #[allow(unsafe_code)]
    pub fn activate(&mut self) -> Result<(), PluginError> {
        if self.active {
            return Ok(());
        }
        unsafe {
            let plugin = &*self.plugin_ptr;
            if plugin
                .activate
                .is_some_and(|act| !act(self.plugin_ptr, self.sample_rate as f64, 32, 4096))
            {
                return Err(PluginError::Activation {
                    plugin_id: self.descriptor.id.clone(),
                    message: "activate returned false".into(),
                });
            }
            if let Some(start) = plugin.start_processing {
                start(self.plugin_ptr);
            }
        }
        self.active = true;
        Ok(())
    }

    /// Deactivate DSP processing.
    #[allow(unsafe_code)]
    pub fn deactivate(&mut self) {
        if !self.active {
            return;
        }
        unsafe {
            let plugin = &*self.plugin_ptr;
            if let Some(stop) = plugin.stop_processing {
                stop(self.plugin_ptr);
            }
            if let Some(deact) = plugin.deactivate {
                deact(self.plugin_ptr);
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
        ctx: &ProcessContext,
    ) -> Result<(), PluginError> {
        let frames = in_l.len().min(in_r.len()).min(out_l.len()).min(out_r.len());
        if frames == 0 {
            return Ok(());
        }

        // Copy input slices to internal preallocated buffers
        self.in_buf_left[..frames].copy_from_slice(&in_l[..frames]);
        self.in_buf_right[..frames].copy_from_slice(&in_r[..frames]);

        let mut in_ptrs = [
            self.in_buf_left.as_mut_ptr(),
            self.in_buf_right.as_mut_ptr(),
        ];
        let mut out_ptrs = [
            self.out_buf_left.as_mut_ptr(),
            self.out_buf_right.as_mut_ptr(),
        ];

        let in_audio = ClapAudioBuffer {
            data32: in_ptrs.as_mut_ptr(),
            data64: std::ptr::null_mut(),
            channel_count: 2,
            latency: 0,
            constant_mask: 0,
        };

        let mut out_audio = ClapAudioBuffer {
            data32: out_ptrs.as_mut_ptr(),
            data64: std::ptr::null_mut(),
            channel_count: 2,
            latency: 0,
            constant_mask: 0,
        };

        let process_data = ClapProcess {
            steady_time: ctx.frame_position as i64,
            frames_count: frames as u32,
            transport: std::ptr::null(),
            audio_inputs: &in_audio,
            audio_outputs: &mut out_audio,
            audio_inputs_count: 1,
            audio_outputs_count: 1,
            in_events: std::ptr::null(),
            out_events: std::ptr::null_mut(),
        };

        unsafe {
            let plugin = &*self.plugin_ptr;
            if let Some(proc_fn) = plugin.process {
                let status = proc_fn(self.plugin_ptr, &process_data);
                if status < 0 {
                    return Err(PluginError::Processing {
                        plugin_id: self.descriptor.id.clone(),
                        message: format!("clap process error code {status}"),
                    });
                }
            }
        }

        out_l[..frames].copy_from_slice(&self.out_buf_left[..frames]);
        out_r[..frames].copy_from_slice(&self.out_buf_right[..frames]);
        Ok(())
    }
}

impl Drop for ActiveClapInstance {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        self.deactivate();
        unsafe {
            let plugin = &*self.plugin_ptr;
            if let Some(destroy_fn) = plugin.destroy {
                destroy_fn(self.plugin_ptr);
            }
        }
    }
}
