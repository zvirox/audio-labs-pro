# `alp-desktop` / `studio_commands`

## Purpose

Extended Tauri IPC command handlers for per-track mixer automation, real hardware/device probing, plugin scanning, neural model integrity verification, voice synthesis, lexicon preview, offline spectral repair, time stretching, loudness analysis, and transcription.

## Public API

- Track mixer commands (`timeline_add_track`, `timeline_set_track_gain`, `timeline_set_track_pan`, `timeline_toggle_track_mute`, `timeline_toggle_track_solo`, `timeline_toggle_track_arm`, `timeline_set_master_gain`)
- Studio bridges (`get_audio_devices`, `get_model_catalogue`, `verify_model_integrity_cmd`, `get_hardware_profile`, `get_discovered_plugins`, `get_voice_profiles`, `desktop_tts_synthesize`, `apply_pronunciation_lexicon`, `dsp_spectral_repair_file`, `dsp_time_stretch_file`, `analyze_audio_file_loudness`, `desktop_transcribe_audio`)

## Data flow

Handlers lock `AppState::session` only briefly for synchronous state mutations (and emit `session_revision_changed`), or delegate offline DSP/voice/hardware operations to `alp_session::studio_bridges`.

## Allowed dependencies

`alp-session`, `tauri`. Contains zero `unsafe` code (`check-deps.sh`).

## Threading / real-time notes

All commands run on Tauri worker threads; real-time audio callbacks remain untouched inside `alp-engine`.

## Error handling

Lock poisoning and backend failures return structured `Result<_, String>` errors to the React UI.

## Testing

Compiled cleanly in `cargo clippy --workspace --all-targets` and tested alongside `alp-desktop` state tests and `alp-session::studio_bridges` unit tests.

## Known limits

- Offline file repair, loudness, and transcription require valid local audio paths accessible to the desktop user.
