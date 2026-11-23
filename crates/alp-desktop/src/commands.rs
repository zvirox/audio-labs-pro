//! Typed Tauri IPC command handlers.
//!
//! Connects the React frontend to the headless `alp_session::EditSession` and
//! `alp_session::PersistedWorkspace`.

use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, State};

use alp_session::{
    AiActivityEntry, AiControlStatus, ApprovalDecision, FrequencyScale, MeterFrame, PanelManager,
    PermissionTier, PersistedWorkspace, ProjectSummary, Size, SpectralSelectionDto,
    SpectrogramConfig, SpectrogramData, TimelineDto, TimelinePanPointDto, TransportController,
    TransportState, WaveformPeaksDto, WindowType,
};

use crate::glass::GlassReport;
use crate::state::AppState;

/// Payload emitted when the project revision changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRevisionPayload {
    pub revision: u64,
    pub is_dirty: bool,
    pub reason: String,
}

/// Retrieve the authoritative summary of the current live editing session.
#[tauri::command]
pub fn get_project_summary(state: State<'_, AppState>) -> Result<ProjectSummary, String> {
    let session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;
    Ok(session.project_summary())
}

/// Retrieve the persisted workspace preferences and panel configuration.
#[tauri::command]
pub fn get_workspace_state(state: State<'_, AppState>) -> Result<PersistedWorkspace, String> {
    let workspace = state
        .workspace
        .lock()
        .map_err(|e| format!("workspace lock poisoned: {e}"))?;
    Ok(workspace.clone())
}

/// Save updated workspace preferences and panel configuration to disk.
#[tauri::command]
pub fn save_workspace_state(
    workspace: PersistedWorkspace,
    state: State<'_, AppState>,
) -> Result<(), String> {
    workspace.save_default()?;
    let mut lock = state
        .workspace
        .lock()
        .map_err(|e| format!("workspace lock poisoned: {e}"))?;
    *lock = workspace;
    Ok(())
}

/// Reset panels to default arrangement and persist to disk.
#[tauri::command]
pub fn reset_workspace_layout(state: State<'_, AppState>) -> Result<PersistedWorkspace, String> {
    let mut lock = state
        .workspace
        .lock()
        .map_err(|e| format!("workspace lock poisoned: {e}"))?;
    let pm = PanelManager::default_for(Size::new(1280.0, 800.0));
    lock.panels = pm.panels().to_vec();
    lock.save_default()?;
    Ok(lock.clone())
}

/// Retrieve the active glass compositor inspection report.
#[tauri::command]
pub fn get_glass_report(state: State<'_, AppState>) -> Result<GlassReport, String> {
    let glass = state
        .glass
        .lock()
        .map_err(|e| format!("glass lock poisoned: {e}"))?;
    Ok(glass.clone())
}

/// Trigger an edit on the live session and emit `session_revision_changed` event.
#[tauri::command]
pub fn trigger_demo_edit(
    app: AppHandle,
    action: String,
    state: State<'_, AppState>,
) -> Result<SessionRevisionPayload, String> {
    let session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;

    let revision = session.revision() + 1;
    let payload = SessionRevisionPayload {
        revision,
        is_dirty: true,
        reason: action,
    };

    app.emit("session_revision_changed", &payload)
        .map_err(|e| format!("failed to emit session_revision_changed: {e}"))?;

    Ok(payload)
}

/// Start audio playback via the transport controller.
#[tauri::command]
pub fn play_transport(state: State<'_, AppState>) -> Result<TransportState, String> {
    let session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;
    let mut transport = state
        .transport
        .lock()
        .map_err(|e| format!("transport lock poisoned: {e}"))?;
    transport.play(&session)
}

/// Pause audio playback, preserving the current playhead position.
#[tauri::command]
pub fn pause_transport(state: State<'_, AppState>) -> Result<TransportState, String> {
    let mut transport = state
        .transport
        .lock()
        .map_err(|e| format!("transport lock poisoned: {e}"))?;
    transport.pause()
}

/// Stop audio playback and rewind playhead.
#[tauri::command]
pub fn stop_transport(state: State<'_, AppState>) -> Result<TransportState, String> {
    let mut transport = state
        .transport
        .lock()
        .map_err(|e| format!("transport lock poisoned: {e}"))?;
    transport.stop()
}

/// Seek audio playhead to a specific position in seconds.
#[tauri::command]
pub fn seek_transport(
    position_secs: f64,
    state: State<'_, AppState>,
) -> Result<TransportState, String> {
    let mut transport = state
        .transport
        .lock()
        .map_err(|e| format!("transport lock poisoned: {e}"))?;
    transport.seek(position_secs)
}

/// Set master output volume (0.0..=1.5).
#[tauri::command]
pub fn set_transport_volume(
    volume: f32,
    state: State<'_, AppState>,
) -> Result<TransportState, String> {
    let mut transport = state
        .transport
        .lock()
        .map_err(|e| format!("transport lock poisoned: {e}"))?;
    transport.set_volume(volume);
    Ok(transport.state())
}

/// Configure loop playback boundaries and state.
#[tauri::command]
pub fn set_transport_loop(
    enabled: bool,
    start_secs: f64,
    end_secs: f64,
    state: State<'_, AppState>,
) -> Result<TransportState, String> {
    let mut transport = state
        .transport
        .lock()
        .map_err(|e| format!("transport lock poisoned: {e}"))?;
    transport.set_loop(enabled, start_secs, end_secs);
    Ok(transport.state())
}

/// Retrieve the current authoritative transport state.
#[tauri::command]
pub fn get_transport_state(state: State<'_, AppState>) -> Result<TransportState, String> {
    let transport = state
        .transport
        .lock()
        .map_err(|e| format!("transport lock poisoned: {e}"))?;
    Ok(transport.state())
}

/// Retrieve the multi-level peak pyramid for the active session.
#[tauri::command]
pub fn get_waveform_peaks(state: State<'_, AppState>) -> Result<WaveformPeaksDto, String> {
    let session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;
    TransportController::build_peaks_for_session(&session)
}

/// Retrieve synthetic 1-hour waveform peaks for the M0/M4 autoscroll gate.
#[tauri::command]
pub fn get_benchmark_1hour_peaks() -> Result<WaveformPeaksDto, String> {
    Ok(TransportController::synthetic_1hour_peaks())
}

/// Stream live calibrated meter frames to the UI over Tauri Channel at ~30 Hz.
#[tauri::command]
pub fn subscribe_meters(
    on_event: Channel<MeterFrame>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let transport_clone = state.transport.clone();
    std::thread::spawn(move || {
        loop {
            let frame = {
                let Ok(mut transport) = transport_clone.lock() else {
                    break;
                };
                transport.tick()
            };
            if on_event.send(frame).is_err() {
                // UI receiver disconnected or closed
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(33));
        }
    });
    Ok(())
}

fn emit_revision(app: &AppHandle, revision: u64, is_dirty: bool, reason: &str) {
    let payload = SessionRevisionPayload {
        revision,
        is_dirty,
        reason: reason.to_string(),
    };
    let _ = app.emit("session_revision_changed", &payload);
}

/// Retrieve the full multitrack timeline state.
#[tauri::command]
pub fn get_timeline_state(state: State<'_, AppState>) -> Result<TimelineDto, String> {
    let session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;
    Ok(session.timeline_dto())
}

/// Undo the last edit operation on the session command stack.
#[tauri::command]
pub fn timeline_undo(app: AppHandle, state: State<'_, AppState>) -> Result<TimelineDto, String> {
    let mut session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;
    session.undo();
    let dto = session.timeline_dto();
    emit_revision(&app, dto.revision, session.is_dirty(), "undo");
    Ok(dto)
}

/// Redo the next undone edit operation on the session command stack.
#[tauri::command]
pub fn timeline_redo(app: AppHandle, state: State<'_, AppState>) -> Result<TimelineDto, String> {
    let mut session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;
    session.redo();
    let dto = session.timeline_dto();
    emit_revision(&app, dto.revision, session.is_dirty(), "redo");
    Ok(dto)
}

/// Split an identified clip at a timestamp in seconds.
#[tauri::command]
pub fn timeline_split_clip(
    app: AppHandle,
    clip_id: String,
    split_secs: f64,
    state: State<'_, AppState>,
) -> Result<TimelineDto, String> {
    let mut session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;
    session.split_clip_at(&clip_id, split_secs)?;
    let dto = session.timeline_dto();
    emit_revision(&app, dto.revision, session.is_dirty(), "split_clip");
    Ok(dto)
}

/// Delete selected range or clip with optional ripple shift.
#[tauri::command]
pub fn timeline_delete_selected(
    app: AppHandle,
    ripple: bool,
    state: State<'_, AppState>,
) -> Result<TimelineDto, String> {
    let mut session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;
    session.delete(ripple);
    let dto = session.timeline_dto();
    emit_revision(&app, dto.revision, session.is_dirty(), "delete");
    Ok(dto)
}

/// Cut selected timeline range to clipboard.
#[tauri::command]
pub fn timeline_cut(
    app: AppHandle,
    ripple: bool,
    state: State<'_, AppState>,
) -> Result<TimelineDto, String> {
    let mut session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;
    session.cut();
    if ripple {
        session.delete(true);
    }
    let dto = session.timeline_dto();
    emit_revision(&app, dto.revision, session.is_dirty(), "cut");
    Ok(dto)
}

/// Copy selected timeline range to clipboard.
#[tauri::command]
pub fn timeline_copy(state: State<'_, AppState>) -> Result<(), String> {
    let mut session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;
    session.copy();
    Ok(())
}

/// Paste clipboard contents into the active track.
#[tauri::command]
pub fn timeline_paste(app: AppHandle, state: State<'_, AppState>) -> Result<TimelineDto, String> {
    let mut session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;
    session.paste();
    let dto = session.timeline_dto();
    emit_revision(&app, dto.revision, session.is_dirty(), "paste");
    Ok(dto)
}

/// Move a clip to a target track and timeline position.
#[tauri::command]
pub fn timeline_move_clip(
    app: AppHandle,
    clip_id: String,
    target_track_id: String,
    new_start_secs: f64,
    state: State<'_, AppState>,
) -> Result<TimelineDto, String> {
    let mut session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;
    session.move_clip_by_id(&clip_id, &target_track_id, new_start_secs)?;
    let dto = session.timeline_dto();
    emit_revision(&app, dto.revision, session.is_dirty(), "move_clip");
    Ok(dto)
}

/// Trim a clip's boundaries non-destructively.
#[tauri::command]
pub fn timeline_trim_clip(
    app: AppHandle,
    clip_id: String,
    new_start_secs: f64,
    new_duration_secs: f64,
    state: State<'_, AppState>,
) -> Result<TimelineDto, String> {
    let mut session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;
    session.trim_clip_by_id(&clip_id, new_start_secs, new_duration_secs)?;
    let dto = session.timeline_dto();
    emit_revision(&app, dto.revision, session.is_dirty(), "trim_clip");
    Ok(dto)
}

/// Set fade-in and fade-out durations on a clip.
#[tauri::command]
pub fn timeline_set_clip_fades(
    app: AppHandle,
    clip_id: String,
    fade_in_secs: f64,
    fade_out_secs: f64,
    state: State<'_, AppState>,
) -> Result<TimelineDto, String> {
    let mut session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;
    session.set_clip_fades_by_id(&clip_id, fade_in_secs, fade_out_secs)?;
    let dto = session.timeline_dto();
    emit_revision(&app, dto.revision, session.is_dirty(), "set_clip_fades");
    Ok(dto)
}

/// Set volume envelope points on a clip.
#[tauri::command]
pub fn timeline_set_clip_envelope(
    app: AppHandle,
    clip_id: String,
    points: Vec<(f64, f32)>,
    state: State<'_, AppState>,
) -> Result<TimelineDto, String> {
    let mut session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;
    session.set_clip_envelope_by_id(&clip_id, points)?;
    let dto = session.timeline_dto();
    emit_revision(&app, dto.revision, session.is_dirty(), "set_clip_envelope");
    Ok(dto)
}

/// Configure or clear loop playback boundaries.
#[tauri::command]
pub fn timeline_set_loop_range(
    app: AppHandle,
    start_secs: f64,
    end_secs: f64,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<TimelineDto, String> {
    let mut session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;
    if enabled {
        session.set_loop_range(start_secs, end_secs);
    } else {
        session.clear_loop_range();
    }
    let dto = session.timeline_dto();
    emit_revision(&app, dto.revision, session.is_dirty(), "set_loop_range");
    Ok(dto)
}

/// Add an annotated cue marker to the timeline.
#[tauri::command]
pub fn timeline_add_marker(
    app: AppHandle,
    time_secs: f64,
    label: String,
    state: State<'_, AppState>,
) -> Result<TimelineDto, String> {
    let mut session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;
    session.add_marker(time_secs, label);
    let dto = session.timeline_dto();
    emit_revision(&app, dto.revision, session.is_dirty(), "add_marker");
    Ok(dto)
}

/// Remove a cue marker by its identifier.
#[tauri::command]
pub fn timeline_remove_marker(
    app: AppHandle,
    marker_id: String,
    state: State<'_, AppState>,
) -> Result<TimelineDto, String> {
    let mut session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;
    if let Ok(id) = marker_id.parse() {
        session.remove_marker(id);
    }
    let dto = session.timeline_dto();
    emit_revision(&app, dto.revision, session.is_dirty(), "remove_marker");
    Ok(dto)
}

/// Select a clip by identifier, or clear clip selection with None.
#[tauri::command]
pub fn timeline_select_clip(
    clip_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<TimelineDto, String> {
    let mut session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;
    if let Some(id_str) = clip_id {
        if let Ok(id) = id_str.parse() {
            session.select_clip(id);
        }
    }
    Ok(session.timeline_dto())
}

/// Set in-out editing selection range in seconds.
#[tauri::command]
pub fn timeline_set_selection(
    start_secs: f64,
    end_secs: f64,
    state: State<'_, AppState>,
) -> Result<TimelineDto, String> {
    let mut session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;
    session.set_selection(start_secs, end_secs);
    Ok(session.timeline_dto())
}

/// Compute high-resolution spectrogram for an audio clip.
#[tauri::command]
pub fn get_clip_spectrogram(
    clip_id: String,
    fft_size: Option<usize>,
    window_type: Option<String>,
    freq_scale: Option<String>,
    num_freq_bins: Option<usize>,
    state: State<'_, AppState>,
) -> Result<SpectrogramData, String> {
    let session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;

    let mut config = SpectrogramConfig::default();
    if let Some(sz) = fft_size {
        config.fft_size = sz;
        config.hop_size = sz / 4;
    }
    if let Some(w) = window_type {
        config.window_type = match w.to_lowercase().as_str() {
            "hamming" => WindowType::Hamming,
            "blackmanharris" | "blackman_harris" => WindowType::BlackmanHarris,
            "rectangular" => WindowType::Rectangular,
            _ => WindowType::Hanning,
        };
    }
    if let Some(s) = freq_scale {
        config.freq_scale = match s.to_lowercase().as_str() {
            "linear" => FrequencyScale::Linear,
            "mel" => FrequencyScale::Mel,
            _ => FrequencyScale::Logarithmic,
        };
    }
    if let Some(bins) = num_freq_bins {
        config.num_freq_bins = bins;
    }

    session.compute_clip_spectrogram_by_id(&clip_id, Some(config))
}

/// Set view mode (waveform, spectrogram, or split) for a track.
#[tauri::command]
pub fn timeline_set_track_view_mode(
    app: AppHandle,
    track_id: String,
    view_mode: String,
    state: State<'_, AppState>,
) -> Result<TimelineDto, String> {
    let mut session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;

    session.set_track_view_mode_by_id(&track_id, &view_mode)?;
    let dto = session.timeline_dto();
    emit_revision(&app, dto.revision, session.is_dirty(), "set_track_view_mode");
    Ok(dto)
}

/// Set or clear 2D spectral selection bounding box for a track.
#[tauri::command]
pub fn timeline_set_spectral_selection(
    app: AppHandle,
    track_id: String,
    selection: Option<SpectralSelectionDto>,
    state: State<'_, AppState>,
) -> Result<TimelineDto, String> {
    let mut session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;

    session.set_spectral_selection_by_id(&track_id, selection)?;
    let dto = session.timeline_dto();
    emit_revision(&app, dto.revision, session.is_dirty(), "set_spectral_selection");
    Ok(dto)
}

/// Set stereo pan automation envelope points for a clip.
#[tauri::command]
pub fn timeline_set_clip_pan_envelope(
    app: AppHandle,
    clip_id: String,
    points: Vec<TimelinePanPointDto>,
    state: State<'_, AppState>,
) -> Result<TimelineDto, String> {
    let mut session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;

    let raw_points: Vec<(f64, f32)> = points
        .into_iter()
        .map(|p| (p.offset_secs, p.pan))
        .collect();
    session.set_clip_pan_envelope_by_id(&clip_id, raw_points)?;
    let dto = session.timeline_dto();
    emit_revision(&app, dto.revision, session.is_dirty(), "set_clip_pan_envelope");
    Ok(dto)
}

/// Slip audio content within fixed clip boundaries.
#[tauri::command]
pub fn timeline_slip_clip(
    app: AppHandle,
    clip_id: String,
    delta_secs: f64,
    state: State<'_, AppState>,
) -> Result<TimelineDto, String> {
    let mut session = state
        .session
        .lock()
        .map_err(|e| format!("session lock poisoned: {e}"))?;

    session.slip_clip_by_id(&clip_id, delta_secs)?;
    let dto = session.timeline_dto();
    emit_revision(&app, dto.revision, session.is_dirty(), "slip_clip");
    Ok(dto)
}

/// Query the current status of the AI control socket and attached agents.
#[tauri::command]
pub fn get_ai_status(state: State<'_, AppState>) -> Result<AiControlStatus, String> {
    let ai = state
        .ai
        .lock()
        .map_err(|e| format!("ai lock poisoned: {e}"))?;
    Ok(ai.status())
}

/// Toggle the AI control socket on/off.
#[tauri::command]
pub fn toggle_ai_control(
    state: State<'_, AppState>,
) -> Result<AiControlStatus, String> {
    let mut ai = state
        .ai
        .lock()
        .map_err(|e| format!("ai lock poisoned: {e}"))?;
    if ai.is_enabled() {
        ai.stop("stopped by operator");
        Ok(ai.status())
    } else {
        let runtime = std::env::var_os("XDG_RUNTIME_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let tier = ai.tier();
        ai.start(&runtime, tier)
    }
}

/// Set the permission tier for attached AI/MCP clients.
#[tauri::command]
pub fn set_ai_permission_tier(
    tier: String,
    state: State<'_, AppState>,
) -> Result<AiControlStatus, String> {
    let mut ai = state
        .ai
        .lock()
        .map_err(|e| format!("ai lock poisoned: {e}"))?;
    let parsed_tier = match tier.to_lowercase().as_str() {
        "observe" => PermissionTier::Observe,
        "export" => PermissionTier::Export,
        _ => PermissionTier::Edit,
    };
    ai.set_tier(parsed_tier);
    Ok(ai.status())
}

/// Respond to an incoming client approval request.
#[tauri::command]
pub fn respond_ai_approval(
    client_id: String,
    decision: String,
    remember: bool,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let mut ai = state
        .ai
        .lock()
        .map_err(|e| format!("ai lock poisoned: {e}"))?;
    let approval_decision = match decision.to_lowercase().as_str() {
        "session" => ApprovalDecision::Session,
        "always" => ApprovalDecision::Always,
        _ => ApprovalDecision::Deny,
    };
    Ok(ai.respond_approval(&client_id, approval_decision, remember))
}

/// Retrieve the recent AI activity and audit logs.
#[tauri::command]
pub fn get_ai_activity_log(
    state: State<'_, AppState>,
) -> Result<Vec<AiActivityEntry>, String> {
    let ai = state
        .ai
        .lock()
        .map_err(|e| format!("ai lock poisoned: {e}"))?;
    Ok(ai.activity_log())
}

/// Convenience undo specifically for AI operations.
#[tauri::command]
pub fn undo_ai_change(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<TimelineDto, String> {
    timeline_undo(app, state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_1hour_peaks_structure() {
        let peaks = get_benchmark_1hour_peaks().expect("peaks succeed");
        assert_eq!(peaks.duration_secs, 3600.0);
        assert_eq!(peaks.sample_rate, 48_000);
        assert_eq!(peaks.levels.len(), 3);
        assert_eq!(peaks.levels[0].pairs.len(), 675_000);
    }

    #[test]
    fn test_app_state_initialization() {
        let state = AppState::new_demo();
        let session = state.session.lock().unwrap();
        assert_eq!(session.project().name, "Phase 4 demo");

        let transport = state.transport.lock().unwrap();
        let t_state = transport.state();
        assert!(!t_state.playing);
        assert_eq!(t_state.position_secs, 0.0);
    }
}
