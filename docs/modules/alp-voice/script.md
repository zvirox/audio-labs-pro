# `alp-voice` / `script`

## Purpose

Provides data structures and rendering logic for Script mode, supporting line-by-line synthesis, granular paragraph regeneration, gapless stitching, and stem exporting.

## Public API

- `LineStatus`: `Pending`, `Rendering`, `Ready`, `Failed`
- `ScriptLine`: `new`, `invalidate`, `set_audio`
- `Script`: `new`, `add_line`, `get_line`, `get_line_mut`, `update_text`, `stitch`, `export_stems`

## Data flow

1. A `Script` is organized into an ordered list of `ScriptLine` entries.
2. When text for a line changes (`update_text`), only that line is invalidated, preserving cached audio for all other lines.
3. `stitch` concatenates cached audio from all ready lines with a configurable inter-line pause into a single `PcmBuffer`.
4. `export_stems` exports individual line takes into sanitized audio files.

## Allowed dependencies

- `alp-core::CoreError`, `alp-media::{ExportSettings, PcmBuffer, export}`
- `serde`, Standard library

## Threading / real-time notes

Script operations run on the worker pool or background tasks.

## Error handling

Returns `CoreError::Validation` if buffer formats cannot be unified, or `CoreError::Io` on filesystem export errors.

## Testing

Unit tests in `script.rs` verify that editing one line invalidates only its cache while preserving others, and that stitching correctly combines buffers with pause padding.

## Known limits

- In-memory audio caching for lines; disk caching can be added for multi-hour book scripts.
