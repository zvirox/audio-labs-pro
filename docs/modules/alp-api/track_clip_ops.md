# `alp-api` / `track_clip_ops`

## Purpose

Provides track property manipulation, clip updating and removal, timeline marker creation, and atomic batch editing operations for `ApiSession`.

## Public API

- `EditBatchReport`: operations applied count, execution status.
- `ApiSession::track_update(&mut self, track_id: &str, name: Option<String>, gain: Option<f32>, pan: Option<f32>, muted: Option<bool>, solo: Option<bool>, armed: Option<bool>) -> Result<(), ApiError>`
- `ApiSession::clip_update(&mut self, clip_id: &str, start_secs: Option<f64>, gain: Option<f32>, fade_in_secs: Option<f64>, fade_out_secs: Option<f64>) -> Result<(), ApiError>`
- `ApiSession::clip_remove(&mut self, clip_id: &str) -> Result<(), ApiError>`
- `ApiSession::marker_add(&mut self, name: &str, position_secs: f64) -> Result<String, ApiError>`
- `ApiSession::edit_batch(&mut self, operations: Vec<serde_json::Value>) -> Result<EditBatchReport, ApiError>`

## Data flow

1. Verifies caller permissions under `PermissionTier::Edit`.
2. Locates target tracks, clips, or timeline markers by ID.
3. Maps requested seconds/dB parameters to sample frames and linear gains using the project's native sample rate.
4. Wraps each mutation in an undoable `alp-core::Command` (`RenameTrack`, `SetTrackGain`, `SetClipFades`, `MoveClip`, `RemoveClip`, `AddMarker`).
5. Appends commands to the session command stack.

## Allowed dependencies

- `serde::{Deserialize, Serialize}`
- `alp_core::{AddMarker, ClipId, MoveClip, RemoveClip, RenameTrack, SetClipFades, SetClipGain, SetTrackGain, SetTrackMuted, SetTrackPan, SetTrackSolo, frames_to_secs, secs_to_frames}`
- `crate::error::ApiError`
- `crate::session::ApiSession`
- `crate::types::PermissionTier`

## Threading / real-time notes

Edits execute on worker/session threads under session lock; changes update project state and propagate safely to the playback engine.

## Error handling

Returns `ApiError::NotFound` if track or clip IDs do not exist; `ApiError::Validation` if IDs or values are malformed.

## Testing

Covered by track update, clip manipulation, marker, and batch editing tests in `alp-api` and `alp-mcp`.

## Known limits

- Batch transactions currently execute operations sequentially; a mid-batch failure rolls back via the checkpoint manager.
