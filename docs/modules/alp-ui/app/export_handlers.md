# `alp-ui` / `app` / `export_handlers`

## Purpose

Handles background audio rendering and multi-format audio preset exports (master mix, podcast, voice-over) from timeline scenes.

## Public API

- `App::export_mix(&mut self)`
- `App::export_with_preset(&mut self, preset: alp_media::ExportPreset)`

## Data flow

1. Extracts the active render `Scene` from `EditSession`.
2. Validates punch-in ranges or timeline boundaries.
3. Spawns an asynchronous job on `JobQueue` to render audio through `alp_engine::Mixer`.
4. Writes encoded audio files (WAV, FLAC, MP3) through `alp_media::export` with progress reporting.
5. Updates export status toasts on the shell interface.

## Allowed dependencies

- `alp_core::{now_ms, AppDirs}`
- `alp_media::{ExportSettings, ExportPreset, PcmBuffer, export}`
- `alp_engine::{DEFAULT_BLOCK_FRAMES, Mixer}`
- `crate::app::App`
- `crate::session::EditSession`

## Threading / real-time notes

Timeline audio rendering is dispatched to background worker threads via `JobQueue`; the UI thread is never blocked during synthesis or disk writes.

## Error handling

If directory creation or audio rendering fails, an error toast is surfaced immediately and recorded in the export queue without panicking.

## Testing

Covered by workspace tests, headless export CLI verification, and acceptance demo scripts.

## Known limits

- Default render output directory is scoped to project `renders/` subfolder.
