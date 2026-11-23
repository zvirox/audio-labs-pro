//! Typed Tauri IPC command handlers.
//!
//! Connects the React frontend to the headless `alp_session::EditSession` and
//! `alp_session::PersistedWorkspace`.

use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, State};

use alp_session::{
    MeterFrame, PanelManager, PersistedWorkspace, ProjectSummary, Size, TransportController,
    TransportState, WaveformPeaksDto,
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
