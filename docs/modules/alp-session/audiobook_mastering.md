# `alp-session` / `audiobook_mastering`

## Purpose

Desktop IPC bridge for audiobook mastering. Decodes chapter/master audio from disk and delegates ACX compliance analysis and native M4B export to `alp_api` and `alp_media`.

## Public API

- `AcxChapterFileDto { title, audio_path }`.
- `M4bChapterDto`, `M4bMetadataDto`, `M4bExportRequestDto`.
- `analyze_acx_files(&[AcxChapterFileDto]) -> Result<AcxQaReport, String>`.
- `export_audiobook_m4b(&M4bExportRequestDto) -> Result<M4bExportReport, String>`.

## Data flow

1. Tauri commands receive DTOs from the React Stories workspace.
2. Source audio is decoded to `PcmBuffer` via `alp_media::decode`.
3. ACX analysis voices borrowed sample slices into `alp_api::analyze_acx_chapters`.
4. M4B export decodes the rendered master, reads optional cover art, and calls `alp_media::export_m4b`.

## Allowed dependencies

`alp-api` (ACX analysis), `alp-media` (decode + M4B writer), `serde` for DTOs. No direct audio-device or UI dependencies.

## Threading / real-time notes

Synchronous file I/O and encoding; intended for Tauri command workers, never the audio callback. Large audiobooks should be invoked from a background job by the caller.

## Error handling

Decode failures, missing cover files, and muxer errors are converted to descriptive strings for Tauri `Result<T, String>` handlers. No partial destination file remains because the muxer writes atomically.

## Testing

A unit test asserts that analysing a missing chapter file returns an error rather than a fabricated report. Muxer behaviour is covered by the `alp-media::m4b` tests.

## Known limits

- Cover art is detected by PNG magic; everything else is treated as JPEG.
- Chapter DTOs carry timing explicitly; they are not derived from markers automatically.
- Master audio must already be rendered and accessible on disk.
