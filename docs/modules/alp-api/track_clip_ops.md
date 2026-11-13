# `alp-api` / `track_clip_ops`

## Purpose

Mutates tracks, clips and markers, and implements the atomic `edit_batch`
operation. Every mutation becomes an undoable `alp-core::Command`, and a batch
either applies every operation or leaves the project untouched, collapsing into
one undo entry.

## Public API

- `EditBatchReport` — `label`, `operations_applied`, `single_undo_step`,
  `status`.
- `BatchOp` — a `#[serde(tag = "op", rename_all = "snake_case")]` enum with:
  - `TrackUpdate { track_id, name?, gain?, pan?, muted?, solo?, armed? }`
  - `ClipUpdate { clip_id, start_secs?, gain?, fade_in_secs?, fade_out_secs? }`
  - `ClipRemove { clip_id }`
  - `ClipSplit { clip_id, split_secs }`
  - `MarkerAdd { name, position_secs }`
  - `RangeDelete { start_secs, end_secs }`
  - `EffectApply { track_id, effect_id, params? }`
  - `EffectRemove { track_id, effect_id }`
- `ApiSession::track_update(&mut self, track_id, name, gain, pan, muted, solo, armed) -> Result<(), ApiError>`
- `ApiSession::clip_update(&mut self, clip_id, start_secs, gain, fade_in_secs, fade_out_secs) -> Result<(), ApiError>`
- `ApiSession::clip_remove(&mut self, clip_id: &str) -> Result<(), ApiError>`
- `ApiSession::marker_add(&mut self, name: &str, position_secs: f64) -> Result<String, ApiError>`
- `ApiSession::edit_batch(&mut self, label: &str, ops: &[BatchOp]) -> Result<EditBatchReport, ApiError>`

## Data flow

1. Check `PermissionTier::Edit`.
2. Resolve the target track/clip/marker by handle (`NotFound` otherwise).
3. Convert seconds to frames with the project sample rate and clamp values.
4. Wrap the mutation in the matching `alp-core` command (`RenameTrack`,
   `SetTrackGain`, `SetTrackPan`, `SetTrackMuted`, `SetTrackSolo`, `MoveClip`,
   `SetClipFades`, `SetClipGain`, `RemoveClip`, `SplitClip`, `AddMarker`,
   `DeleteRange`, `SetTrackEffects`) and apply it through `execute_command`,
   which advances the revision and records an audit entry.

`edit_batch` records `undo_len` and the revision before the loop. Each item is
routed through `apply_batch_op`. On the first failure it undoes every command
added since the batch started, restores the revision, and returns
`"... no changes were applied"`. On success it calls
`group_commands_since("Batch edit", undo_before)`, which collapses the N
commands into one `CompositeCommand` undo entry and returns whether anything was
grouped.

## Allowed dependencies

- `serde::{Deserialize, Serialize}`
- `alp_core::{AddMarker, ClipId, CommandStack, DeleteRange, FrameRange, MoveClip, RemoveClip, RenameTrack, SetClipFades, SetClipGain, SetTrackGain, SetTrackMuted, SetTrackPan, SetTrackSolo, SplitClip, frames_to_secs, secs_to_frames}`
- `crate::error::ApiError`
- `crate::session::ApiSession`
- `crate::types::PermissionTier`

## Threading / real-time notes

Synchronous on the caller's thread under the session lock. Commands mutate the
in-memory project; the playback engine reads the updated project on its next
block. No allocation or locking occurs on the audio callback.

## Error handling

- `ApiError::NotFound` when a track, clip, effect or marker handle is unknown.
- `ApiError::Validation` for malformed values, an empty batch, or a batch that
  failed part-way (the message names the failing index and states that nothing
  stuck).
- Rollback failure is not expected but is tolerated: the loop stops undoing at
  the first reversal error rather than panicking.

## Testing

Unit tests in `track_clip_ops.rs`:

- `a_batch_applies_every_operation_as_one_undo_step`
- `a_failing_batch_leaves_the_project_untouched`

MCP end-to-end: `edit_batch_applies_real_operations_and_collapses_into_one_undo`
(asserts a real rename lands and one undo reverts it) and
`a_failing_edit_batch_reports_no_changes`.

Integration: `a_batch_of_edits_is_atomic` in
`crates/alp-api/tests/export_and_script.rs`.

## Known limits

- `range_delete` ripples on the **first** track only; it does not ripple across
  all tracks.
- A batch is capped only by memory; there is no per-batch operation limit beyond
  the session's command stack.
- Effects inserted through `EffectApply` are recorded on the chain but the
  offline renderer does not process them yet, so a batch that adds an effect
  changes the project, not the audio.
