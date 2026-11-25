# `alp-desktop::commands`

## Purpose

Defines the Tauri `#[tauri::command]` IPC handlers bridging the React Linear UI frontend (`frontend/src/ipc/client.ts`) to `alp-session`, `alp-engine`, and `alp-mcp`.

## Public API

- Session & Workspace: `get_project_summary`, `get_workspace_state`, `save_workspace_state`, `reset_workspace_layout`, `trigger_demo_edit`
- Window Surface: `get_glass_report` (reports opaque solid Linear surface status and desktop session details)
- Timeline & Waveform: `get_timeline_state`, `dispatch_timeline_command`, `get_waveform_peaks`, `get_spectrogram_data`, `apply_spectral_selection`
- Transport & Engine: `get_transport_state`, `transport_play`, `transport_pause`, `transport_stop`, `transport_seek`, `transport_set_volume`, `transport_toggle_loop`, `get_audio_devices`
- Voice, Capture, Models & Effects: `get_voice_profiles`, `create_voice_profile`, `synthesize_tts`, `get_stories`, `get_captures`, `get_transcript`, `get_effects_chain`, `get_discovered_plugins`, `get_model_catalogue`
- AI & MCP Host: `get_ai_status`, `toggle_ai_control`, `set_ai_permission_tier`, `undo_ai_change`, `get_ai_activity_log`, `respond_ai_approval`

## Data flow

Frontend components invoke `invoke()` helpers; handlers lock `AppState`, mutate or snapshot `alp-session`, broadcast live events (`session_revision`, `ai_activity`, `ai_client_approval`), and return JSON DTOs.

## Allowed dependencies

- `tauri`, `serde`, `serde_json`, `crate::state::AppState`, `alp-session`, `alp-api`

## Threading / real-time notes

Handlers execute on Tauri's async/command runtime and never block audio callbacks.

## Error handling

Errors serialize deterministically as `IpcError { code, message }`.

## Testing

Covered by `alp-desktop` and `alp-session` unit tests plus frontend IPC contract types (`frontend/src/ipc/types.ts`).

## Known limits

Standalone Vite browser previews (`npm run preview`) run without Tauri internals and automatically fall back to deterministic in-memory fixtures.
