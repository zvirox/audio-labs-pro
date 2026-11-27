//! VST3 bundle discovery and COM ABI hosting engine (`vst3` v3.8+ Linux x86_64).
//!
//! Loads `.vst3` shared libraries via `libloading`, negotiates `ModuleEntry` /
//! `GetPluginFactory` / `IPluginFactory` / `IComponent` / `IAudioProcessor` /
//! `IEditController`, executes 32-bit float audio blocks with sample-accurate
//! parameter changes, and manages X11 `IPlugView` editor embedding lifecycle.

use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use libloading::os::unix::Library as UnixLibrary;
use libloading::{Library, Symbol};
use vst3::Steinberg::Vst::BusDirections_::{kInput, kOutput};
use vst3::Steinberg::Vst::MediaTypes_::kAudio;
use vst3::Steinberg::Vst::ParameterInfo_::ParameterFlags_;
use vst3::Steinberg::Vst::ProcessModes_::kRealtime;
use vst3::Steinberg::Vst::SpeakerArr::kStereo;
use vst3::Steinberg::Vst::SymbolicSampleSizes_::kSample32;
use vst3::Steinberg::Vst::{
    AudioBusBuffers, AudioBusBuffers__type0, IAudioProcessor, IAudioProcessorTrait, IComponent,
    IComponentHandler, IComponentHandlerTrait, IComponentTrait, IEditController,
    IEditControllerTrait, IHostApplication, IHostApplicationTrait, IParamValueQueue,
    IParamValueQueueTrait, IParameterChanges, IParameterChangesTrait, ParamID, ParamValue,
    ParameterInfo, ProcessData, ProcessSetup, SpeakerArrangement, String128,
};
use vst3::Steinberg::{
    FUnknown, IPlugFrame, IPlugFrameTrait, IPlugView, IPlugViewTrait, IPluginBaseTrait,
    IPluginFactory, IPluginFactory2, IPluginFactory2Trait, IPluginFactoryTrait, PClassInfo,
    PClassInfo2, PFactoryInfo, TUID, ViewRect, char8, int32, kNotImplemented,
    kPlatformTypeX11EmbedWindowID, kResultOk, kResultTrue, tresult,
};
use vst3::{Class, ComPtr, ComWrapper, Interface};

use crate::error::PluginError;
use crate::types::{PluginCategory, PluginDescriptor, PluginFormat, ProcessContext};

/// Maximum recursion depth inside a VST3 bundle.
const MAX_BUNDLE_DEPTH: usize = 4;
/// Maximum frame block size prepared during `setupProcessing`.
const MAX_VST3_BLOCK_FRAMES: usize = 4096;
/// Preallocated parameter change queues for lock-free audio blocks.
const MAX_PARAM_QUEUES: usize = 64;
/// VST3 module class category tag for audio processors/effects.
const AUDIO_MODULE_CLASS: &str = "Audio Module Class";

type ModuleEntryFn = unsafe extern "C" fn(shared_library_handle: *mut c_void) -> bool;
type ModuleExitFn = unsafe extern "C" fn() -> bool;
type GetPluginFactoryFn = unsafe extern "C" fn() -> *mut IPluginFactory;

#[derive(Default)]
struct Vst3HostApp;

impl Class for Vst3HostApp {
    type Interfaces = (IHostApplication,);
}

impl IHostApplicationTrait for Vst3HostApp {
    unsafe fn getName(&self, name: *mut String128) -> tresult {
        if name.is_null() {
            return vst3::Steinberg::kInvalidArgument;
        }
        let mut buf: String128 = [0; 128];
        for (dst, ch) in buf.iter_mut().zip("Audio Labs Pro".encode_utf16()) {
            *dst = ch;
        }
        unsafe { *name = buf };
        kResultOk
    }

    unsafe fn createInstance(
        &self,
        _cid: *mut TUID,
        _iid: *mut TUID,
        obj: *mut *mut c_void,
    ) -> tresult {
        if !obj.is_null() {
            unsafe { *obj = std::ptr::null_mut() };
        }
        kNotImplemented
    }
}

#[derive(Default)]
struct HostComponentHandler;

impl Class for HostComponentHandler {
    type Interfaces = (IComponentHandler,);
}

impl IComponentHandlerTrait for HostComponentHandler {
    unsafe fn beginEdit(&self, _id: ParamID) -> tresult {
        kResultOk
    }

    unsafe fn performEdit(&self, _id: ParamID, _value_normalized: ParamValue) -> tresult {
        kResultOk
    }

    unsafe fn endEdit(&self, _id: ParamID) -> tresult {
        kResultOk
    }

    unsafe fn restartComponent(&self, _flags: int32) -> tresult {
        kResultOk
    }
}

#[derive(Default)]
struct HostPlugFrame {
    last_rect: Mutex<Option<(i32, i32, i32, i32)>>,
}

impl Class for HostPlugFrame {
    type Interfaces = (IPlugFrame,);
}

impl IPlugFrameTrait for HostPlugFrame {
    unsafe fn resizeView(&self, _view: *mut IPlugView, new_size: *mut ViewRect) -> tresult {
        if !new_size.is_null()
            && let Ok(mut guard) = self.last_rect.lock()
        {
            let rect = unsafe { *new_size };
            *guard = Some((rect.left, rect.top, rect.right, rect.bottom));
        }
        kResultOk
    }
}

struct ParamQueueState {
    param_id: ParamID,
    offset: i32,
    value: ParamValue,
    has_point: bool,
}

impl Default for ParamQueueState {
    fn default() -> Self {
        Self {
            param_id: 0,
            offset: 0,
            value: 0.0,
            has_point: false,
        }
    }
}

struct HostParamValueQueue {
    state: UnsafeCell<ParamQueueState>,
}

impl HostParamValueQueue {
    fn new() -> Self {
        Self {
            state: UnsafeCell::new(ParamQueueState::default()),
        }
    }

    unsafe fn reset(&self, param_id: ParamID, value: ParamValue) {
        let s = unsafe { &mut *self.state.get() };
        s.param_id = param_id;
        s.offset = 0;
        s.value = value;
        s.has_point = true;
    }
}

// Safety: Mutated exclusively on the owning audio/host instance before process().
unsafe impl Send for HostParamValueQueue {}
unsafe impl Sync for HostParamValueQueue {}

impl Class for HostParamValueQueue {
    type Interfaces = (IParamValueQueue,);
}

impl IParamValueQueueTrait for HostParamValueQueue {
    unsafe fn getParameterId(&self) -> ParamID {
        unsafe { (*self.state.get()).param_id }
    }

    unsafe fn getPointCount(&self) -> int32 {
        i32::from(unsafe { (*self.state.get()).has_point })
    }

    unsafe fn getPoint(
        &self,
        index: int32,
        sample_offset: *mut int32,
        value: *mut ParamValue,
    ) -> tresult {
        let s = unsafe { &*self.state.get() };
        if index != 0 || !s.has_point || sample_offset.is_null() || value.is_null() {
            return vst3::Steinberg::kInvalidArgument;
        }
        unsafe {
            *sample_offset = s.offset;
            *value = s.value;
        }
        kResultOk
    }

    unsafe fn addPoint(
        &self,
        sample_offset: int32,
        value: ParamValue,
        index: *mut int32,
    ) -> tresult {
        let s = unsafe { &mut *self.state.get() };
        s.offset = sample_offset;
        s.value = value;
        s.has_point = true;
        if !index.is_null() {
            unsafe { *index = 0 };
        }
        kResultOk
    }
}

struct HostParameterChanges {
    queues: Vec<ComWrapper<HostParamValueQueue>>,
    active_count: UnsafeCell<usize>,
}

impl HostParameterChanges {
    fn new() -> Self {
        let queues = (0..MAX_PARAM_QUEUES)
            .map(|_| ComWrapper::new(HostParamValueQueue::new()))
            .collect();
        Self {
            queues,
            active_count: UnsafeCell::new(0),
        }
    }

    fn set_changes(&self, changes: &[(u32, f64)]) {
        let count = changes.len().min(self.queues.len());
        unsafe { *self.active_count.get() = count };
        for (slot, &(param_id, value)) in self.queues.iter().zip(changes.iter()).take(count) {
            unsafe { slot.reset(param_id, value) };
        }
    }

    fn clear(&self) {
        unsafe { *self.active_count.get() = 0 };
    }
}

// Safety: Accessed synchronously inside `&mut ActiveVst3Instance` blocks only.
unsafe impl Send for HostParameterChanges {}
unsafe impl Sync for HostParameterChanges {}

impl Class for HostParameterChanges {
    type Interfaces = (IParameterChanges,);
}

impl IParameterChangesTrait for HostParameterChanges {
    unsafe fn getParameterCount(&self) -> int32 {
        unsafe { *self.active_count.get() as int32 }
    }

    unsafe fn getParameterData(&self, index: int32) -> *mut IParamValueQueue {
        if index < 0 {
            return std::ptr::null_mut();
        }
        let count = unsafe { *self.active_count.get() };
        let idx = index as usize;
        if idx >= count {
            return std::ptr::null_mut();
        }
        self.queues[idx]
            .as_com_ref::<IParamValueQueue>()
            .map_or(std::ptr::null_mut(), |r| r.as_ptr())
    }

    unsafe fn addParameterData(
        &self,
        id: *const ParamID,
        index: *mut int32,
    ) -> *mut IParamValueQueue {
        if id.is_null() {
            return std::ptr::null_mut();
        }
        let count = unsafe { *self.active_count.get() };
        if count >= self.queues.len() {
            return std::ptr::null_mut();
        }
        unsafe {
            *self.active_count.get() = count + 1;
            self.queues[count].reset(*id, 0.0);
            if !index.is_null() {
                *index = count as int32;
            }
        }
        self.queues[count]
            .as_com_ref::<IParamValueQueue>()
            .map_or(std::ptr::null_mut(), |r| r.as_ptr())
    }
}

/// Metadata recovered for one VST3 plugin parameter.
#[derive(Debug, Clone)]
pub struct Vst3ParamMeta {
    /// Native VST3 parameter identifier.
    pub param_id: u32,
    /// Stable normalized symbol used in `ActivePlugin`.
    pub symbol: String,
    /// Display name.
    pub name: String,
    /// Unit string (if any).
    pub unit: String,
    /// Step count (`0` = continuous normalized `0.0..=1.0`, `1` = toggle).
    pub step_count: i32,
    /// Default normalized value (`0.0..=1.0`).
    pub default_normalized: f64,
    /// Whether parameter is marked read-only.
    pub read_only: bool,
    /// Whether parameter represents plugin bypass.
    pub is_bypass: bool,
}

/// Attached native X11 editor handle for a live VST3 instance.
pub struct ActiveVst3View {
    view: ComPtr<IPlugView>,
    _frame: ComWrapper<HostPlugFrame>,
    attached: bool,
    /// Preferred editor dimensions `(width, height)`.
    pub size: (u32, u32),
}

impl ActiveVst3View {
    fn attach(
        controller: &ComPtr<IEditController>,
        parent_xid: u32,
        plugin_id: &str,
    ) -> Result<Self, PluginError> {
        let editor_name = b"editor\0";
        let raw_view = unsafe { controller.createView(editor_name.as_ptr().cast()) };
        let view =
            unsafe { ComPtr::from_raw(raw_view) }.ok_or_else(|| PluginError::Activation {
                plugin_id: plugin_id.to_owned(),
                message: "VST3 plugin does not expose an editor view".into(),
            })?;

        let supported = unsafe { view.isPlatformTypeSupported(kPlatformTypeX11EmbedWindowID) };
        if supported != kResultOk && supported != kResultTrue {
            return Err(PluginError::AbiMismatch {
                plugin_id: plugin_id.to_owned(),
                message: "VST3 editor view does not support X11EmbedWindowID".into(),
            });
        }

        let frame = ComWrapper::new(HostPlugFrame::default());
        let frame_ptr = frame
            .as_com_ref::<IPlugFrame>()
            .map_or(std::ptr::null_mut(), |r| r.as_ptr());
        unsafe { view.setFrame(frame_ptr) };

        let mut rect = ViewRect {
            left: 0,
            top: 0,
            right: 640,
            bottom: 400,
        };
        let _ = unsafe { view.getSize(&mut rect) };
        let width = u32::try_from((rect.right - rect.left).max(1)).unwrap_or(640);
        let height = u32::try_from((rect.bottom - rect.top).max(1)).unwrap_or(400);

        let attached_res = unsafe {
            view.attached(
                parent_xid as usize as *mut c_void,
                kPlatformTypeX11EmbedWindowID,
            )
        };
        if attached_res != kResultOk && attached_res != kResultTrue {
            return Err(PluginError::Activation {
                plugin_id: plugin_id.to_owned(),
                message: format!("VST3 IPlugView::attached failed (code {attached_res})"),
            });
        }

        Ok(Self {
            view,
            _frame: frame,
            attached: true,
            size: (width, height),
        })
    }

    fn detach(&mut self) {
        if self.attached {
            let _ = unsafe { self.view.removed() };
            let _ = unsafe { self.view.setFrame(std::ptr::null_mut()) };
            self.attached = false;
        }
    }
}

impl Drop for ActiveVst3View {
    fn drop(&mut self) {
        self.detach();
    }
}

/// A live instantiated VST3 plugin wrapped over its C++ COM vtable interfaces.
pub struct ActiveVst3Instance {
    descriptor: PluginDescriptor,
    params: Vec<Vst3ParamMeta>,
    symbol_to_param_id: HashMap<String, u32>,
    pending_param_changes: Vec<(u32, f64)>,
    in_buf_left: Vec<f32>,
    in_buf_right: Vec<f32>,
    out_buf_left: Vec<f32>,
    out_buf_right: Vec<f32>,
    editor_view: Option<ActiveVst3View>,
    // Drop COM objects BEFORE Library & ModuleExit so vtable memory remains valid.
    controller_separate: bool,
    controller: Option<ComPtr<IEditController>>,
    processor: Option<ComPtr<IAudioProcessor>>,
    component: Option<ComPtr<IComponent>>,
    param_changes: ComWrapper<HostParameterChanges>,
    _handler: ComWrapper<HostComponentHandler>,
    _host_app: ComWrapper<Vst3HostApp>,
    module_exit: Option<ModuleExitFn>,
    _library: Option<Library>,
}

// Safety: Access to COM pointers happens only under mutable host ownership on one thread at a time.
unsafe impl Send for ActiveVst3Instance {}

impl ActiveVst3Instance {
    /// Load and activate a VST3 plugin from its bundle binary.
    pub fn load(desc: &PluginDescriptor, sample_rate: u32) -> Result<Self, PluginError> {
        let lib = unsafe { Library::new(&desc.path) }.map_err(|e| PluginError::LibraryLoad {
            path: desc.path.clone(),
            message: e.to_string(),
        })?;

        let (lib, module_exit) = enter_vst3_module(lib);
        let factory = query_plugin_factory(&lib, &desc.path)?;
        let class_cid = select_audio_module_cid(&factory, &desc.name, &desc.path)?;

        let host_app = ComWrapper::new(Vst3HostApp);
        let handler = ComWrapper::new(HostComponentHandler);
        let param_changes = ComWrapper::new(HostParameterChanges::new());

        let mut comp_raw: *mut c_void = std::ptr::null_mut();
        let res = unsafe {
            factory.createInstance(
                class_cid.as_ptr(),
                IComponent::IID.as_ptr().cast(),
                &mut comp_raw,
            )
        };
        if res != kResultOk || comp_raw.is_null() {
            drop(factory);
            exit_vst3_module(module_exit, lib);
            return Err(PluginError::Instantiation {
                plugin_id: desc.id.clone(),
                message: format!("IPluginFactory::createInstance(IComponent) failed ({res})"),
            });
        }
        let component =
            unsafe { ComPtr::<IComponent>::from_raw(comp_raw.cast()) }.ok_or_else(|| {
                PluginError::Instantiation {
                    plugin_id: desc.id.clone(),
                    message: "createInstance returned null IComponent".into(),
                }
            })?;

        let host_unknown = host_app
            .as_com_ref::<FUnknown>()
            .map_or(std::ptr::null_mut(), |r| r.as_ptr());
        let init_res = unsafe { component.initialize(host_unknown) };
        if init_res != kResultOk && init_res != kResultTrue {
            return Err(PluginError::Instantiation {
                plugin_id: desc.id.clone(),
                message: format!("IComponent::initialize failed (code {init_res})"),
            });
        }

        let processor =
            component
                .cast::<IAudioProcessor>()
                .ok_or_else(|| PluginError::AbiMismatch {
                    plugin_id: desc.id.clone(),
                    message: "VST3 component does not implement IAudioProcessor".into(),
                })?;

        let mut in_arr: SpeakerArrangement = kStereo;
        let mut out_arr: SpeakerArrangement = kStereo;
        let _ = unsafe { processor.setBusArrangements(&mut in_arr, 1, &mut out_arr, 1) };
        let _ = unsafe { component.activateBus(kAudio as int32, kInput as int32, 0, 1) };
        let _ = unsafe { component.activateBus(kAudio as int32, kOutput as int32, 0, 1) };

        let mut setup = ProcessSetup {
            processMode: kRealtime as int32,
            symbolicSampleSize: kSample32 as int32,
            maxSamplesPerBlock: MAX_VST3_BLOCK_FRAMES as int32,
            sampleRate: f64::from(sample_rate),
        };
        let setup_res = unsafe { processor.setupProcessing(&mut setup) };
        if setup_res != kResultOk && setup_res != kResultTrue {
            return Err(PluginError::Activation {
                plugin_id: desc.id.clone(),
                message: format!("IAudioProcessor::setupProcessing failed (code {setup_res})"),
            });
        }
        let active_res = unsafe { component.setActive(1) };
        if active_res != kResultOk && active_res != kResultTrue {
            return Err(PluginError::Activation {
                plugin_id: desc.id.clone(),
                message: format!("IComponent::setActive failed (code {active_res})"),
            });
        }
        let _ = unsafe { processor.setProcessing(1) };

        let (controller, controller_separate) =
            resolve_edit_controller(&factory, &component, host_unknown);
        let mut params = Vec::new();
        let mut symbol_to_param_id = HashMap::new();
        if let Some(ref ctrl) = controller {
            if let Some(h) = handler.as_com_ref::<IComponentHandler>() {
                let _ = unsafe { ctrl.setComponentHandler(h.as_ptr()) };
            }
            let count = unsafe { ctrl.getParameterCount() }.max(0);
            for index in 0..count {
                let mut info = zeroed_param_info();
                if unsafe { ctrl.getParameterInfo(index, &mut info) } == kResultOk {
                    let meta = param_meta_from_info(&info, index);
                    symbol_to_param_id.insert(meta.symbol.clone(), meta.param_id);
                    params.push(meta);
                }
            }
        }

        drop(factory);

        Ok(Self {
            descriptor: desc.clone(),
            params,
            symbol_to_param_id,
            pending_param_changes: Vec::with_capacity(MAX_PARAM_QUEUES),
            in_buf_left: vec![0.0; MAX_VST3_BLOCK_FRAMES],
            in_buf_right: vec![0.0; MAX_VST3_BLOCK_FRAMES],
            out_buf_left: vec![0.0; MAX_VST3_BLOCK_FRAMES],
            out_buf_right: vec![0.0; MAX_VST3_BLOCK_FRAMES],
            editor_view: None,
            controller_separate,
            controller,
            processor: Some(processor),
            component: Some(component),
            param_changes,
            _handler: handler,
            _host_app: host_app,
            module_exit,
            _library: Some(lib),
        })
    }

    /// Discovered parameter metadata.
    #[must_use]
    pub fn parameters(&self) -> &[Vst3ParamMeta] {
        &self.params
    }

    /// Update a normalized VST3 parameter (`0.0..=1.0`) and queue it for DSP.
    pub fn set_parameter_normalized(
        &mut self,
        symbol: &str,
        normalized: f64,
    ) -> Result<(), PluginError> {
        let param_id =
            *self
                .symbol_to_param_id
                .get(symbol)
                .ok_or_else(|| PluginError::UnknownParameter {
                    plugin_id: self.descriptor.id.clone(),
                    param_id: 0,
                })?;
        let clamped = normalized.clamp(0.0, 1.0);
        if let Some(ref ctrl) = self.controller {
            let _ = unsafe { ctrl.setParamNormalized(param_id, clamped) };
        }
        if let Some(entry) = self
            .pending_param_changes
            .iter_mut()
            .find(|(id, _)| *id == param_id)
        {
            entry.1 = clamped;
        } else if self.pending_param_changes.len() < MAX_PARAM_QUEUES {
            self.pending_param_changes.push((param_id, clamped));
        }
        Ok(())
    }

    /// Read current normalized parameter value from `IEditController`.
    #[must_use]
    pub fn get_parameter_normalized(&self, param_id: u32) -> Option<f64> {
        self.controller
            .as_ref()
            .map(|ctrl| unsafe { ctrl.getParamNormalized(param_id) })
    }

    /// Execute one real-time stereo block via `IAudioProcessor::process`.
    pub fn process_block(
        &mut self,
        in_l: &[f32],
        in_r: &[f32],
        out_l: &mut [f32],
        out_r: &mut [f32],
        ctx: &ProcessContext,
    ) -> Result<(), PluginError> {
        let frames = in_l
            .len()
            .min(in_r.len())
            .min(out_l.len())
            .min(out_r.len())
            .min(MAX_VST3_BLOCK_FRAMES);
        if frames == 0 {
            return Ok(());
        }

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

        let mut in_bus = AudioBusBuffers {
            numChannels: 2,
            silenceFlags: 0,
            __field0: AudioBusBuffers__type0 {
                channelBuffers32: in_ptrs.as_mut_ptr(),
            },
        };
        let mut out_bus = AudioBusBuffers {
            numChannels: 2,
            silenceFlags: 0,
            __field0: AudioBusBuffers__type0 {
                channelBuffers32: out_ptrs.as_mut_ptr(),
            },
        };

        self.param_changes.set_changes(&self.pending_param_changes);
        self.pending_param_changes.clear();
        let param_changes_ptr = self
            .param_changes
            .as_com_ref::<IParameterChanges>()
            .map_or(std::ptr::null_mut(), |r| r.as_ptr());

        let mut vst_ctx = unsafe { std::mem::zeroed::<vst3::Steinberg::Vst::ProcessContext>() };
        vst_ctx.sampleRate = f64::from(ctx.sample_rate);
        vst_ctx.projectTimeSamples = ctx.frame_position as i64;
        if ctx.is_playing {
            vst_ctx.state = vst3::Steinberg::Vst::ProcessContext_::StatesAndFlags_::kPlaying;
        }

        let mut data = ProcessData {
            processMode: kRealtime as int32,
            symbolicSampleSize: kSample32 as int32,
            numSamples: frames as int32,
            numInputs: 1,
            numOutputs: 1,
            inputs: &mut in_bus,
            outputs: &mut out_bus,
            inputParameterChanges: param_changes_ptr,
            outputParameterChanges: std::ptr::null_mut(),
            inputEvents: std::ptr::null_mut(),
            outputEvents: std::ptr::null_mut(),
            processContext: &mut vst_ctx,
        };

        if let Some(ref processor) = self.processor {
            let status = unsafe { processor.process(&mut data) };
            self.param_changes.clear();
            if status != kResultOk && status != kResultTrue {
                return Err(PluginError::Processing {
                    plugin_id: self.descriptor.id.clone(),
                    message: format!("VST3 IAudioProcessor::process failed (code {status})"),
                });
            }
        }

        out_l[..frames].copy_from_slice(&self.out_buf_left[..frames]);
        out_r[..frames].copy_from_slice(&self.out_buf_right[..frames]);
        Ok(())
    }

    /// Create and attach the VST3 native X11 editor view to `parent_xid`.
    ///
    /// Returns the preferred `(width, height)` reported by `IPlugView::getSize`.
    pub fn attach_editor_x11(&mut self, parent_xid: u32) -> Result<(u32, u32), PluginError> {
        if let Some(ref view) = self.editor_view {
            return Ok(view.size);
        }
        let ctrl = self
            .controller
            .as_ref()
            .ok_or_else(|| PluginError::Activation {
                plugin_id: self.descriptor.id.clone(),
                message: "VST3 plugin exposes no IEditController for editor UI".into(),
            })?;
        let attached = ActiveVst3View::attach(ctrl, parent_xid, &self.descriptor.id)?;
        let size = attached.size;
        self.editor_view = Some(attached);
        Ok(size)
    }

    /// Detach and release any active native X11 editor view.
    pub fn detach_editor_x11(&mut self) {
        if let Some(mut view) = self.editor_view.take() {
            view.detach();
        }
    }
}

impl Drop for ActiveVst3Instance {
    fn drop(&mut self) {
        self.detach_editor_x11();
        if let Some(ref processor) = self.processor {
            let _ = unsafe { processor.setProcessing(0) };
        }
        if let Some(ref component) = self.component {
            let _ = unsafe { component.setActive(0) };
        }
        if self.controller_separate
            && let Some(ref controller) = self.controller
        {
            let _ = unsafe { controller.terminate() };
        }
        if let Some(ref component) = self.component {
            let _ = unsafe { component.terminate() };
        }
        self.controller = None;
        self.processor = None;
        self.component = None;
        if let Some(lib) = self._library.take() {
            exit_vst3_module(self.module_exit, lib);
        }
    }
}

fn enter_vst3_module(lib: Library) -> (Library, Option<ModuleExitFn>) {
    let module_exit = unsafe {
        lib.get::<ModuleExitFn>(b"ModuleExit\0")
            .ok()
            .map(|sym| *sym)
    };
    let module_entry = unsafe {
        lib.get::<ModuleEntryFn>(b"ModuleEntry\0")
            .ok()
            .map(|sym| *sym)
    };
    if let Some(entry_fn) = module_entry {
        let unix_lib: UnixLibrary = lib.into();
        let raw_handle = unix_lib.into_raw();
        let _ = unsafe { entry_fn(raw_handle) };
        let restored = unsafe { UnixLibrary::from_raw(raw_handle) };
        (restored.into(), module_exit)
    } else {
        (lib, module_exit)
    }
}

fn exit_vst3_module(module_exit: Option<ModuleExitFn>, lib: Library) {
    if let Some(exit_fn) = module_exit {
        let _ = unsafe { exit_fn() };
    }
    drop(lib);
}

fn query_plugin_factory(lib: &Library, path: &Path) -> Result<ComPtr<IPluginFactory>, PluginError> {
    let get_factory: Symbol<GetPluginFactoryFn> = unsafe { lib.get(b"GetPluginFactory\0") }
        .map_err(|_| PluginError::MissingSymbol {
            path: path.to_path_buf(),
            symbol: "GetPluginFactory".into(),
        })?;
    let raw = unsafe { get_factory() };
    unsafe { ComPtr::from_raw(raw) }.ok_or_else(|| PluginError::Discovery {
        path: path.to_path_buf(),
        message: "GetPluginFactory returned null".into(),
    })
}

fn resolve_edit_controller(
    factory: &ComPtr<IPluginFactory>,
    component: &ComPtr<IComponent>,
    host_unknown: *mut FUnknown,
) -> (Option<ComPtr<IEditController>>, bool) {
    if let Some(ctrl) = component.cast::<IEditController>() {
        return (Some(ctrl), false);
    }
    let mut ctrl_cid: TUID = [0; 16];
    if unsafe { component.getControllerClassId(&mut ctrl_cid) } != kResultOk {
        return (None, false);
    }
    let mut ctrl_raw: *mut c_void = std::ptr::null_mut();
    let res = unsafe {
        factory.createInstance(
            ctrl_cid.as_ptr(),
            IEditController::IID.as_ptr().cast(),
            &mut ctrl_raw,
        )
    };
    if res != kResultOk || ctrl_raw.is_null() {
        return (None, false);
    }
    let Some(ctrl) = (unsafe { ComPtr::<IEditController>::from_raw(ctrl_raw.cast()) }) else {
        return (None, false);
    };
    let _ = unsafe { ctrl.initialize(host_unknown) };
    (Some(ctrl), true)
}

fn select_audio_module_cid(
    factory: &ComPtr<IPluginFactory>,
    target_name: &str,
    path: &Path,
) -> Result<TUID, PluginError> {
    let count = unsafe { factory.countClasses() };
    let mut fallback: Option<TUID> = None;
    for index in 0..count {
        let mut info = unsafe { std::mem::zeroed::<PClassInfo>() };
        if unsafe { factory.getClassInfo(index, &mut info) } != kResultOk {
            continue;
        }
        let category = cchars_to_string(&info.category);
        if category != AUDIO_MODULE_CLASS {
            continue;
        }
        let name = cchars_to_string(&info.name);
        if name == target_name {
            return Ok(info.cid);
        }
        if fallback.is_none() {
            fallback = Some(info.cid);
        }
    }
    fallback.ok_or_else(|| PluginError::Discovery {
        path: path.to_path_buf(),
        message: "no Audio Module Class found in VST3 factory".into(),
    })
}

fn cchars_to_string(buf: &[char8]) -> String {
    let bytes: &[u8] = unsafe { std::slice::from_raw_parts(buf.as_ptr().cast::<u8>(), buf.len()) };
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_owned()
}

fn utf16_to_string(buf: &String128) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end]).trim().to_owned()
}

fn zeroed_param_info() -> ParameterInfo {
    ParameterInfo {
        id: 0,
        title: [0; 128],
        shortTitle: [0; 128],
        units: [0; 128],
        stepCount: 0,
        defaultNormalizedValue: 0.0,
        unitId: 0,
        flags: 0,
    }
}

fn param_meta_from_info(info: &ParameterInfo, index: int32) -> Vst3ParamMeta {
    let title = utf16_to_string(&info.title);
    let short_title = utf16_to_string(&info.shortTitle);
    let display = if !title.is_empty() {
        title
    } else if !short_title.is_empty() {
        short_title.clone()
    } else {
        format!("Param {}", info.id)
    };
    let raw_sym = if !short_title.is_empty() {
        short_title
    } else {
        display.clone()
    };
    let sanitized: String = raw_sym
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = sanitized.trim_matches('_');
    let symbol = if trimmed.is_empty() {
        format!("param_{index}")
    } else {
        trimmed.to_owned()
    };
    Vst3ParamMeta {
        param_id: info.id,
        symbol,
        name: display,
        unit: utf16_to_string(&info.units),
        step_count: info.stepCount,
        default_normalized: info.defaultNormalizedValue.clamp(0.0, 1.0),
        read_only: (info.flags & ParameterFlags_::kIsReadOnly) != 0,
        is_bypass: (info.flags & ParameterFlags_::kIsBypass) != 0,
    }
}

/// Scan a `.vst3` bundle directory and return a descriptor for its binary.
///
/// When the `.so` exposes a real VST3 `GetPluginFactory`, factory/class
/// metadata (`name`, `vendor`, `version`, `category`) is populated directly
/// from `IPluginFactory` / `IPluginFactory2`. Dummy or unreadable binaries still
/// fall back to bundle filename discovery so offline scanner checks succeed.
pub fn scan_vst3_bundle(path: &Path) -> Result<PluginDescriptor, PluginError> {
    if !path.is_dir() {
        return Err(PluginError::Discovery {
            path: path.to_path_buf(),
            message: "VST3 bundle is not a directory".into(),
        });
    }
    let binary = find_shared_library(path, 0).ok_or_else(|| PluginError::Discovery {
        path: path.to_path_buf(),
        message: "no shared library found under Contents/<arch>".into(),
    })?;
    let bundle_name = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "VST3 Plugin".to_owned());

    if let Some(probed) = probe_vst3_factory(&binary, &bundle_name) {
        return Ok(probed);
    }

    Ok(PluginDescriptor {
        id: format!("vst3:{bundle_name}"),
        name: bundle_name,
        vendor: "VST3".to_owned(),
        version: "unknown".to_owned(),
        format: PluginFormat::Vst3,
        category: PluginCategory::Effect,
        path: binary,
        audio_inputs: 2,
        audio_outputs: 2,
        param_count: 0,
    })
}

fn probe_vst3_factory(binary: &Path, bundle_name: &str) -> Option<PluginDescriptor> {
    let lib = unsafe { Library::new(binary) }.ok()?;
    let (lib, module_exit) = enter_vst3_module(lib);
    let factory = query_plugin_factory(&lib, binary).ok()?;

    let mut factory_info = unsafe { std::mem::zeroed::<PFactoryInfo>() };
    let default_vendor = if unsafe { factory.getFactoryInfo(&mut factory_info) } == kResultOk {
        let v = cchars_to_string(&factory_info.vendor);
        if v.is_empty() { "VST3".to_owned() } else { v }
    } else {
        "VST3".to_owned()
    };

    let factory2 = factory.cast::<IPluginFactory2>();
    let count = unsafe { factory.countClasses() };
    let mut found: Option<PluginDescriptor> = None;

    for index in 0..count {
        if let Some(ref f2) = factory2 {
            let mut info2 = unsafe { std::mem::zeroed::<PClassInfo2>() };
            if unsafe { f2.getClassInfo2(index, &mut info2) } == kResultOk
                && cchars_to_string(&info2.category) == AUDIO_MODULE_CLASS
            {
                let name = cchars_to_string(&info2.name);
                let vendor = cchars_to_string(&info2.vendor);
                let version = cchars_to_string(&info2.version);
                let sub = cchars_to_string(&info2.subCategories).to_lowercase();
                found = Some(PluginDescriptor {
                    id: format!("vst3:{bundle_name}"),
                    name: if name.is_empty() {
                        bundle_name.to_owned()
                    } else {
                        name
                    },
                    vendor: if vendor.is_empty() {
                        default_vendor.clone()
                    } else {
                        vendor
                    },
                    version: if version.is_empty() {
                        "1.0.0".to_owned()
                    } else {
                        version
                    },
                    format: PluginFormat::Vst3,
                    category: categorize_vst3(&sub),
                    path: binary.to_path_buf(),
                    audio_inputs: 2,
                    audio_outputs: 2,
                    param_count: 0,
                });
                break;
            }
        }
        let mut info = unsafe { std::mem::zeroed::<PClassInfo>() };
        if unsafe { factory.getClassInfo(index, &mut info) } == kResultOk
            && cchars_to_string(&info.category) == AUDIO_MODULE_CLASS
        {
            let name = cchars_to_string(&info.name);
            found = Some(PluginDescriptor {
                id: format!("vst3:{bundle_name}"),
                name: if name.is_empty() {
                    bundle_name.to_owned()
                } else {
                    name
                },
                vendor: default_vendor.clone(),
                version: "1.0.0".to_owned(),
                format: PluginFormat::Vst3,
                category: PluginCategory::Effect,
                path: binary.to_path_buf(),
                audio_inputs: 2,
                audio_outputs: 2,
                param_count: 0,
            });
            break;
        }
    }

    drop(factory2);
    drop(factory);
    exit_vst3_module(module_exit, lib);
    found
}

fn categorize_vst3(sub: &str) -> PluginCategory {
    if sub.contains("instrument") || sub.contains("synth") {
        PluginCategory::Instrument
    } else if sub.contains("restoration") {
        PluginCategory::Restoration
    } else if sub.contains("analyzer") {
        PluginCategory::Analyzer
    } else if sub.contains("tools") || sub.contains("utility") {
        PluginCategory::Utility
    } else {
        PluginCategory::Effect
    }
}

fn find_shared_library(dir: &Path, depth: usize) -> Option<PathBuf> {
    if depth > MAX_BUNDLE_DEPTH {
        return None;
    }
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect();
    entries.sort();
    for entry in entries {
        if entry.is_file() && entry.extension().is_some_and(|ext| ext == "so") {
            return Some(entry);
        }
    }
    for entry in std::fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
    {
        if entry.is_dir()
            && let Some(found) = find_shared_library(&entry, depth + 1)
        {
            return Some(found);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance::ActivePlugin;

    #[test]
    fn non_directory_is_rejected() {
        let error = scan_vst3_bundle(Path::new("/definitely/not/a.bundle")).expect_err("error");
        assert!(error.to_string().contains("not a directory"));
    }

    #[test]
    fn discovers_bundle_binary_and_fails_cleanly_on_invalid_binary() {
        let dir = tempfile::tempdir().expect("tempdir");
        let bundle = dir.path().join("Test.vst3");
        let binary_dir = bundle.join("Contents/x86_64-linux");
        std::fs::create_dir_all(&binary_dir).expect("bundle dirs");
        let binary = binary_dir.join("Test.so");
        std::fs::write(&binary, b"not a real library").expect("binary");
        let descriptor = scan_vst3_bundle(&bundle).expect("descriptor");
        assert_eq!(descriptor.format, PluginFormat::Vst3);
        assert_eq!(descriptor.id, "vst3:Test");
        assert_eq!(descriptor.path, binary);
        assert!(ActivePlugin::instantiate(&descriptor, 48_000).is_err());
    }
}
