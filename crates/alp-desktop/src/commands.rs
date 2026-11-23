//! Typed Tauri IPC command handlers.
//!
//! Connects the React frontend to the headless `alp_session::EditSession` and
//! `alp_session::PersistedWorkspace`.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use alp_session::{PanelManager, PersistedWorkspace, ProjectSummary, Size};

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
