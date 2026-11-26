//! Audio Labs Pro - Tauri 2 Native Desktop Shell Entrypoint
//!
//! Hand-written code contains zero `unsafe`. All window operations use safe
//! Tauri 2, `raw-window-handle 0.6`, and `x11rb`.

mod commands;
mod glass;
mod state;

use alp_session::AiEvent;
use tauri::{Emitter, Manager};

use crate::commands::{
    SessionRevisionPayload, effects_get_response, get_ai_activity_log, get_ai_status,
    get_benchmark_1hour_peaks, get_clip_spectrogram, get_glass_report, get_project_summary,
    get_timeline_state, get_transport_state, get_waveform_peaks, get_workspace_state,
    pause_transport, play_transport, reset_workspace_layout, respond_ai_approval,
    save_workspace_state, seek_transport, set_ai_permission_tier, set_transport_loop,
    set_transport_volume, stop_transport, subscribe_meters, timeline_add_marker, timeline_copy,
    timeline_cut, timeline_delete_selected, timeline_move_clip, timeline_paste, timeline_redo,
    timeline_remove_marker, timeline_select_clip, timeline_set_clip_envelope,
    timeline_set_clip_fades, timeline_set_clip_pan_envelope, timeline_set_loop_range,
    timeline_set_selection, timeline_set_spectral_selection, timeline_set_track_view_mode,
    timeline_slip_clip, timeline_split_clip, timeline_trim_clip, timeline_undo, toggle_ai_control,
    trigger_demo_edit, undo_ai_change,
};
use crate::glass::probe;
use crate::state::AppState;

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("Starting Audio Labs Pro desktop shell (Phase M7)...");

    let app_state = AppState::new_demo();
    let state_for_setup = app_state.clone();

    tauri::Builder::default()
        .setup(move |app| {
            if let Some(window) = app.get_webview_window("main") {
                let report = probe(&window);
                tracing::info!("Compositor glass report: {}", report);
                if let Ok(mut glass) = state_for_setup.glass.lock() {
                    *glass = report;
                }
            } else {
                tracing::warn!("Main webview window not found during setup");
            }

            // AI control event loop: polls socket & MCP completions and emits UI events
            let handle = app.handle().clone();
            let state_for_tick = state_for_setup.clone();
            std::thread::Builder::new()
                .name("alp-ai-event-pump".into())
                .spawn(move || {
                    loop {
                        std::thread::sleep(std::time::Duration::from_millis(20));
                        let events = {
                            let Ok(mut ai) = state_for_tick.ai.lock() else {
                                continue;
                            };
                            let Ok(mut session) = state_for_tick.session.lock() else {
                                continue;
                            };
                            ai.tick(&mut session)
                        };
                        for event in events {
                            match event {
                                AiEvent::RevisionChanged(rev) => {
                                    let is_dirty =
                                        state_for_tick.session.lock().is_ok_and(|s| s.is_dirty());
                                    let payload = SessionRevisionPayload {
                                        revision: rev,
                                        is_dirty,
                                        reason: "ai_mcp_edit".into(),
                                    };
                                    let _ = handle.emit("session_revision_changed", &payload);
                                }
                                AiEvent::ApprovalRequired { client_id } => {
                                    let _ = handle.emit(
                                        "ai_client_approval",
                                        serde_json::json!({ "clientId": client_id }),
                                    );
                                }
                                AiEvent::Activity(entry) => {
                                    let _ = handle.emit("ai_activity_event", &entry);
                                }
                            }
                        }
                    }
                })
                .expect("spawn ai pump");

            Ok(())
        })
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_project_summary,
            get_workspace_state,
            save_workspace_state,
            reset_workspace_layout,
            get_glass_report,
            trigger_demo_edit,
            play_transport,
            pause_transport,
            stop_transport,
            seek_transport,
            set_transport_volume,
            set_transport_loop,
            get_transport_state,
            get_waveform_peaks,
            effects_get_response,
            get_benchmark_1hour_peaks,
            subscribe_meters,
            get_timeline_state,
            timeline_undo,
            timeline_redo,
            timeline_split_clip,
            timeline_delete_selected,
            timeline_cut,
            timeline_copy,
            timeline_paste,
            timeline_move_clip,
            timeline_trim_clip,
            timeline_set_clip_fades,
            timeline_set_clip_envelope,
            timeline_set_loop_range,
            timeline_add_marker,
            timeline_remove_marker,
            timeline_select_clip,
            timeline_set_selection,
            get_clip_spectrogram,
            timeline_set_track_view_mode,
            timeline_set_spectral_selection,
            timeline_set_clip_pan_envelope,
            timeline_slip_clip,
            get_ai_status,
            toggle_ai_control,
            set_ai_permission_tier,
            respond_ai_approval,
            get_ai_activity_log,
            undo_ai_change,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Audio Labs Pro desktop shell");
}
