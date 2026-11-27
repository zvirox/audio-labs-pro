//! Extended Tauri IPC commands for track mixing, hardware, plugins, models, voice, and DSP repair.
//!
//! Keeps `commands.rs` concise and delegates all backend logic cleanly to `alp_session`.

use alp_session::{
    AudioDeviceDto, DesktopTtsReportDto, DesktopVoiceProfileDto, DiscoveredPluginDto,
    FileLoudnessDto, HardwareProfileDto, LexiconPreviewDto, LexiconRuleDto, ModelCatalogueItemDto,
    SpectralRepairReport, SpectralRepairRequestDto, TimeStretchReport, TimeStretchRequestDto,
    TimelineDto, TranscriptionReport, analyze_file_loudness, desktop_spectral_repair_file,
    desktop_time_stretch_file, desktop_transcribe_audio as bridge_transcribe_audio,
    discover_system_plugins, list_audio_devices, list_model_catalogue, list_voice_profiles,
    preview_lexicon_markup, probe_hardware_profile, synthesize_tts_preview, verify_model_integrity,
};
use tauri::{AppHandle, State};

use crate::commands::emit_revision;
use crate::state::AppState;

/// Add a named audio track to the live multitrack session.
#[tauri::command]
pub fn timeline_add_track(
    app: AppHandle,
    name: String,
    state: State<'_, AppState>,
) -> Result<TimelineDto, String> {
    let mut session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;
    session.add_track(&name)?;
    let dto = session.timeline_dto();
    emit_revision(&app, dto.revision, session.is_dirty(), "add_track");
    Ok(dto)
}

/// Set linear gain (0.0..=2.0) on a track by index.
#[tauri::command]
pub fn timeline_set_track_gain(
    app: AppHandle,
    track_index: usize,
    gain: f32,
    state: State<'_, AppState>,
) -> Result<TimelineDto, String> {
    let mut session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;
    session.set_track_gain(track_index, gain);
    let dto = session.timeline_dto();
    emit_revision(&app, dto.revision, session.is_dirty(), "set_track_gain");
    Ok(dto)
}

/// Set stereo pan (-1.0..=1.0) on a track by index.
#[tauri::command]
pub fn timeline_set_track_pan(
    app: AppHandle,
    track_index: usize,
    pan: f32,
    state: State<'_, AppState>,
) -> Result<TimelineDto, String> {
    let mut session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;
    session.set_track_pan(track_index, pan);
    let dto = session.timeline_dto();
    emit_revision(&app, dto.revision, session.is_dirty(), "set_track_pan");
    Ok(dto)
}

/// Toggle mute state on a track by index.
#[tauri::command]
pub fn timeline_toggle_track_mute(
    app: AppHandle,
    track_index: usize,
    state: State<'_, AppState>,
) -> Result<TimelineDto, String> {
    let mut session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;
    session.toggle_track_mute(track_index);
    let dto = session.timeline_dto();
    emit_revision(&app, dto.revision, session.is_dirty(), "toggle_track_mute");
    Ok(dto)
}

/// Toggle solo state on a track by index.
#[tauri::command]
pub fn timeline_toggle_track_solo(
    app: AppHandle,
    track_index: usize,
    state: State<'_, AppState>,
) -> Result<TimelineDto, String> {
    let mut session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;
    session.toggle_track_solo(track_index);
    let dto = session.timeline_dto();
    emit_revision(&app, dto.revision, session.is_dirty(), "toggle_track_solo");
    Ok(dto)
}

/// Toggle record-arm state on a track by index.
#[tauri::command]
pub fn timeline_toggle_track_arm(
    app: AppHandle,
    track_index: usize,
    state: State<'_, AppState>,
) -> Result<TimelineDto, String> {
    let mut session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;
    session.toggle_track_arm(track_index);
    let dto = session.timeline_dto();
    emit_revision(&app, dto.revision, session.is_dirty(), "toggle_track_arm");
    Ok(dto)
}

/// Set master bus linear gain (0.0..=4.0).
#[tauri::command]
pub fn timeline_set_master_gain(
    app: AppHandle,
    gain: f32,
    state: State<'_, AppState>,
) -> Result<TimelineDto, String> {
    let mut session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;
    session.set_master_gain(gain);
    let dto = session.timeline_dto();
    emit_revision(&app, dto.revision, session.is_dirty(), "set_master_gain");
    Ok(dto)
}

/// Enumerate real ALSA / PipeWire audio output and input devices.
#[tauri::command]
pub fn get_audio_devices() -> Result<Vec<AudioDeviceDto>, String> {
    Ok(list_audio_devices())
}

/// List neural model catalogue entries and on-disk verification states.
#[tauri::command]
pub fn get_model_catalogue() -> Result<Vec<ModelCatalogueItemDto>, String> {
    Ok(list_model_catalogue())
}

/// Verify SHA-256 integrity of an installed neural model.
#[tauri::command]
pub fn verify_model_integrity_cmd(model_id: String) -> Result<ModelCatalogueItemDto, String> {
    verify_model_integrity(&model_id)
}

/// Probe host CPU, RAM, and GPU telemetry.
#[tauri::command]
pub fn get_hardware_profile() -> Result<HardwareProfileDto, String> {
    Ok(probe_hardware_profile())
}

/// Scan and return real Linux CLAP, LV2, and VST3 plugins.
#[tauri::command]
pub fn get_discovered_plugins() -> Result<Vec<DiscoveredPluginDto>, String> {
    discover_system_plugins()
}

/// List stored local voice profiles.
#[tauri::command]
pub fn get_voice_profiles() -> Result<Vec<DesktopVoiceProfileDto>, String> {
    list_voice_profiles()
}

/// Synthesize speech into a WAV take using the active voice backend.
#[tauri::command]
pub fn desktop_tts_synthesize(
    voice_id: String,
    text: String,
    speed: f32,
) -> Result<DesktopTtsReportDto, String> {
    synthesize_tts_preview(&voice_id, &text, speed)
}

/// Preview pronunciation lexicon substitution and inline markup parsing.
#[tauri::command]
pub fn apply_pronunciation_lexicon(
    rules: Vec<LexiconRuleDto>,
    text: String,
) -> Result<LexiconPreviewDto, String> {
    Ok(preview_lexicon_markup(rules, &text))
}

/// Run offline spectral notch/harmonic repair on an audio file.
#[tauri::command]
pub fn dsp_spectral_repair_file(
    request: SpectralRepairRequestDto,
) -> Result<SpectralRepairReport, String> {
    desktop_spectral_repair_file(&request)
}

/// Run offline WSOLA pitch-preserving time stretch on an audio file.
#[tauri::command]
pub fn dsp_time_stretch_file(request: TimeStretchRequestDto) -> Result<TimeStretchReport, String> {
    desktop_time_stretch_file(&request)
}

/// Measure integrated LUFS, sample peak, and true peak of an audio file.
#[tauri::command]
pub fn analyze_audio_file_loudness(path: String) -> Result<FileLoudnessDto, String> {
    analyze_file_loudness(&path)
}

/// Transcribe an audio file with word-level timestamps.
#[tauri::command]
pub fn desktop_transcribe_audio(path: String) -> Result<TranscriptionReport, String> {
    bridge_transcribe_audio(&path)
}
