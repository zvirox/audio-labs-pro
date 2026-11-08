# `alp-api` / `export_ops`

## Purpose

Handles delivery presets queries, per-track stems export, transcript export, and localized timeline preview rendering for `ApiSession`.

## Public API

- `PresetListReport`: collection of available delivery presets.
- `PresetEntry`: ID, human-readable label, description, sample rate, bit depth.
- `StemExportReport`: track ID, destination output path, rendered duration.
- `TranscriptExportReport`: format and destination path.
- `PreviewRenderResult`: start time, duration, sample rate, channels, peak amplitude.
- `ApiSession::export_presets(&self) -> Result<PresetListReport, ApiError>`
- `ApiSession::export_stems(&mut self, output_dir: &str, preset_id: &str) -> Result<Vec<StemExportReport>, ApiError>`
- `ApiSession::export_transcript(&self, format: &str, output_path: &str) -> Result<TranscriptExportReport, ApiError>`
- `ApiSession::preview_render(&self, start_secs: f64, duration_secs: f64) -> Result<PreviewRenderResult, ApiError>`

## Data flow

1. Validates caller authorization against `PermissionTier::Observe` or `PermissionTier::Export`.
2. Validates output file destinations within the configured `WorkspaceSandbox`.
3. Renders project timeline:
   - For stems: isolates individual tracks and exports individual audio files.
   - For transcript: extracts spoken text and writes formatted `.txt`, `.srt`, or `.vtt` output.
   - For preview: bounds rendering to requested time window and computes audio metrics.
4. Returns typed operation reports with output paths and metadata.

## Allowed dependencies

- `serde::{Deserialize, Serialize}`
- `alp_media::{ExportPreset, export}`
- `crate::error::ApiError`
- `crate::session::ApiSession`
- `crate::types::PermissionTier`

## Threading / real-time notes

Export operations are offline non-real-time jobs executed on worker threads; audio playback on the real-time engine is unaffected.

## Error handling

Returns `ApiError::Validation` if output paths escape sandbox constraints or contain invalid presets; `ApiError::Audio` on export failures.

## Testing

Covered by export preset and stem generation tests in `alp-api` and MCP server tests.

## Known limits

- Stems export renders sequentially rather than utilizing a parallel background thread pool.
