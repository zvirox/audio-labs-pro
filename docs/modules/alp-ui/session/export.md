# `alp-ui` / `session/export`

## Purpose

Prepares export scenes, handles export paths and provides preset configurations
for background render jobs.

## Public API

- `EditSession::scene(&self) -> Result<MixScene, String>`
- `EditSession::set_export_queued(&mut self, path: &Path)`
- `EditSession::export_settings() -> ExportSettings`
- `EditSession::export_preset_settings(preset: ExportPreset) -> ExportSettings`

## Data flow

Views trigger an export message (`ExportMix` or `ExportPreset`). The shell obtains
an immutable `MixScene` snapshot from `EditSession::scene()` and queues the background
render in the worker job queue with settings configured for WAV or a delivery preset.

## Allowed dependencies

The parent `session` module, `alp-engine::MixScene`, `alp-media::{AudioMetadata, ExportPreset, ExportSettings}`.

## Threading / real-time notes

`MixScene` creation extracts an immutable snapshot on the UI thread. The actual
mixing, resampling, dither and file I/O run entirely on background worker threads.

## Error handling

If the scene cannot be built from the project and sources, an error is returned and
displayed in the status bar/toast without modifying project state.

## Testing

Covered in `session/tests.rs` by verifying `export_preset_settings` output for delivery targets.

## Known limits

- Direct in-app preview of export metadata before queueing is not yet rendered in a modal.
