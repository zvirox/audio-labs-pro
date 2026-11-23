//! Audio Labs Pro - Tauri 2 Native Desktop Shell Entrypoint
//!
//! Hand-written code contains zero `unsafe`. All window operations use safe
//! Tauri 2, `raw-window-handle 0.6`, and `x11rb`.

mod commands;
mod glass;
mod state;

use tauri::Manager;

use crate::commands::{
    get_benchmark_1hour_peaks, get_glass_report, get_project_summary, get_timeline_state,
    get_transport_state, get_waveform_peaks, get_workspace_state, pause_transport, play_transport,
    reset_workspace_layout, save_workspace_state, seek_transport, set_transport_loop,
    set_transport_volume, stop_transport, subscribe_meters, timeline_add_marker, timeline_copy,
    timeline_cut, timeline_delete_selected, timeline_move_clip, timeline_paste, timeline_redo,
    timeline_remove_marker, timeline_select_clip, timeline_set_clip_envelope,
    timeline_set_clip_fades, timeline_set_loop_range, timeline_set_selection, timeline_split_clip,
    timeline_trim_clip, timeline_undo, trigger_demo_edit,
};
use crate::glass::probe;
use crate::state::AppState;

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("Starting Audio Labs Pro desktop shell (Phase M6)...");

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
        ])
        .run(tauri::generate_context!())
        .expect("error while running Audio Labs Pro desktop shell");
}
