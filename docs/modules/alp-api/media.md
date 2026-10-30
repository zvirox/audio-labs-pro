# `alp-api` / `media`

## Purpose

Media tool methods on `ApiSession` for the Phase 10 MCP surface: listing project sources and importing sandboxed audio into the project media directory
(ADR 0027). Clip placement (`clip_add`) complements them from `session.rs`; together they complete the generate-to-timeline chain.

## Public API

- `MediaSummary`, `MediaListReport`: one source summary (id, name, relative path, rate, channels, frames, duration, peak, on-disk presence) and the
  `media_list` result.
- `MediaImportReport`: source id, display name, relative and absolute path, duration, rate, channels, peak for `media_import`.
- `ClipPlacementReport`: clip id, track, source, start, duration, source offset, applied gain for `clip_add` (method lives on `ApiSession` in
  `session.rs`).
- `ApiSession::media_list(&self)` (Observe).
- `ApiSession::media_import(&mut self, path, name?)` (Edit).
- `ApiSession::clip_add(&mut self, track_id, source_id, start_secs, duration_secs?, source_offset_secs?, gain?) -> Result<ClipPlacementReport, ApiError>`
  (Edit; defined in `session.rs` beside the other timeline operations).

## Data flow

1. Calls validate the session tier (`media_list` Observe; import/clip Edit) and are audited through the standard `execute_command` path.
2. `media_list` reads `project.sources` only - no decoding; on-disk presence comes from `ProjectPaths::resolve_relative` existence.
3. `media_import` validates the input through `WorkspaceSandbox::validate_path` (any workspace root, not only the project), decodes it to prove it is
   audio, re-encodes Float32 WAV to `media/{source_id}-{slug}.wav` (fresh `SourceId`, so it can never overwrite existing media), measures the peak, and
   registers the source with the undoable `AddSource` command.
4. `clip_add` clamps and validates start/offset/duration/gain, resolves the source and track (plain-language `NotFound` otherwise), computes the source
   range in the source's own sample rate and the timeline start in the project rate, and appends the clip with the undoable `AddClip` command.

## Allowed dependencies

- `alp-core` (`AddSource`, `Source`, `SourceId`, `frames_to_secs`, `secs_to_frames`, `FrameRange`, plus `AddClip`/`Clip` for `clip_add`)
- `alp-media` (`decode`, `write_wav`, `WavBitDepth`)
- `serde`/`serde_json` (report serialization)
- `crate::error::ApiError`, `crate::session::ApiSession`, `crate::types::PermissionTier`

## Threading / real-time notes

Synchronous file decoding/encoding on the MCP session thread; no audio-device or audio-thread interaction. Expand commands (`AddSource`, `AddClip`) hold
only a cloned record and an index, so undo/redo stays allocation-light.

## Error handling

- Sandbox violations (`..`, symlinked ancestors, outside roots) return `ApiError::SandboxEscape`.
- Non-audio or missing files surface the underlying `alp-media` decode error (`ApiError::Media`).
- Unknown source/track ids return `ApiError::NotFound` with a pointer to `media_list`/`track_list`.
- Negative or non-finite start/offset, non-positive duration, and non-finite gain return `ApiError::Validation`; gain values in range are clamped to
  `0.0..=4.0`; durations longer than the remaining source are clamped to it.

## Testing

Inline `mod tests` in `crates/alp-api/src/media.rs` and the MCP-level tests in `crates/alp-mcp/src/main.rs` cover: the generate-to-import chain with an
undo/redo round trip, one file per import (no overwrites), default naming from the file stem, non-audio and traversal rejections, Observe-tier denial,
full-source default placement with duration matching, undoable placement with the source retained, legal overlaps, gain clamping, and unknown-id handling.

## Known limits

- Imports re-encode to Float32 WAV, so compressed inputs (FLAC) grow on disk; a passthrough-copy mode is a future optimization.
- `clip_add` appends only; moving or resizing placed clips needs `clip_update`/`clip_remove` (later slice).
- Overlaps are legal and summed by the mix engine; callers detect them from the returned range - no conflict warning is emitted yet.
- Generated TTS files are not auto-registered; agents run `media_import` explicitly (ADR 0027 rationale).
