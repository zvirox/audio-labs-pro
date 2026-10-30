# `alp-api` / `session`

## Purpose

The primary automation session coordinator managing project state, undoable commands, timeline tracks/clips, audio analysis, and master rendering.

## Public API

- `ApiSession::new(project, paths, tier, sandbox, audit, client_id) -> Result<Self, ApiError>`
- `ApiSession::project_info(&self) -> Result<ProjectSummary, ApiError>`
- `ApiSession::save_project(&mut self) -> Result<(), ApiError>`
- `ApiSession::undo(&mut self) -> Result<bool, ApiError>`
- `ApiSession::redo(&mut self) -> Result<bool, ApiError>`
- `ApiSession::create_checkpoint(&mut self, label) -> Result<String, ApiError>`
- `ApiSession::restore_checkpoint(&mut self, id) -> Result<(), ApiError>`
- `ApiSession::rollback_session(&mut self) -> Result<bool, ApiError>`
- `ApiSession::list_tracks(&self) -> Result<Vec<TrackSummary>, ApiError>`
- `ApiSession::add_track(&mut self, name) -> Result<String, ApiError>`
- `ApiSession::remove_track(&mut self, track_id) -> Result<(), ApiError>`
- `ApiSession::split_clip(&mut self, clip_id, split_secs) -> Result<(), ApiError>`
- `ApiSession::ripple_delete_range(&mut self, start_secs, end_secs) -> Result<(), ApiError>`
- `ApiSession::clip_add(&mut self, track_id, source_id, start_secs, duration_secs?, source_offset_secs?, gain?) -> Result<ClipPlacementReport, ApiError>` (Phase 10 `clip_add`; appended via the undoable `AddClip` command; see `media.md`)
- `ApiSession::analyze_loudness(&self) -> Result<AnalysisReport, ApiError>`
- `ApiSession::export_audio(&self, preset, target_path) -> Result<String, ApiError>`

## Data flow

1. Verifies caller permission tier before dispatching actions.
2. Applies domain mutations using undoable `alp-core` commands.
3. Automatically increments project mutation revision counters.
4. Records structured audit logs and executes disk operations via sandbox.

## Allowed dependencies

- `alp_core`
- `alp_media`
- `alp_dsp`
- `alp_engine`
- `alp_voice`
- `crate::{audit, checkpoint, error, jobs, sandbox, types}`

## Threading / real-time notes

Session operations execute synchronously or dispatch to background jobs; they do not interact with the audio callback thread.

## Error handling

Returns structured `ApiError` for concurrency conflicts, permission rejections, and validation failures.

## Testing

Covered by unit tests in `session.rs` and automated integration tests in `alp-mcp`.

## Known limits

- Single session write coordinator.
