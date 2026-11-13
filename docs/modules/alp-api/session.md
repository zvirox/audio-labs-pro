# `alp-api` / `session`

## Purpose

The automation session: it owns the project, the undo stack, the revision
counter, permissions, sandbox, audit log, checkpoints, and the job manager. All
typed operations in the other `alp-api` modules are implemented on `ApiSession`,
and it is the single render path used by analysis and export.

## Public API

Construction and state:

- `ApiSession::new(project, paths, tier, sandbox, audit, client_id) -> Result<Self, ApiError>` — creates the session-start checkpoint.
- `check_permission(&self, required: PermissionTier) -> Result<(), ApiError>`
- `project_info(&self) -> Result<ProjectSummary, ApiError>`
- `save_project(&mut self) -> Result<(), ApiError>`
- `undo(&mut self) -> Result<bool, ApiError>` / `redo(&mut self) -> Result<bool, ApiError>`
- `create_checkpoint(&mut self, label) -> Result<String, ApiError>`
- `restore_checkpoint(&mut self, id) -> Result<(), ApiError>`
- `rollback_session(&mut self) -> Result<bool, ApiError>`
- `stage_arguments(&mut self, args: &Value)` / `verify_revision(&self, expected: Option<u64>)`

Editing and rendering:

- `execute_command(&mut self, tool: &str, cmd: Box<dyn Command>) -> Result<(), ApiError>` — applies a command, advances `revision`, marks dirty, records audit.
- `group_commands_since(&mut self, label: &'static str, undo_before: usize) -> bool` (`pub(crate)`) — collapses everything applied since `undo_before` into one `CompositeCommand`; returns `false` when nothing was applied.
- `render_project_mixdown(&self) -> Result<PcmBuffer, ApiError>` (`pub(crate)`) — the one render path.
- `render_track_mixdown(&self, track_id: &str) -> Result<(TrackId, PcmBuffer), ApiError>` (`pub(crate)`) — renders one track in isolation.
- `analyze_loudness(&self) -> Result<AnalysisReport, ApiError>`
- `export_audio(&mut self, preset: ExportPreset, target: impl AsRef<Path>) -> Result<String, ApiError>`

The struct fields `stack`, `revision` and `jobs` are `pub(crate)` so sibling
modules can measure undo depth, record grouped commands, and attach job ids
without exposing them outside the crate.

## Data flow

1. Every public operation checks the permission tier first.
2. Mutations build an `alp-core::Command` and pass it to `execute_command`,
   which runs it against the project, bumps `revision`, sets `dirty`, and writes
   an audit entry with the pre-mutation revision.
3. Multi-command operations call `group_commands_since` to collapse the new
   stack entries into one undo entry (`edit_batch`, `tts_script_generate`,
   `tts_regenerate`).
4. Reads that need audio call `render_project_mixdown`, which decodes each
   source, builds `alp_engine::MixScene::from_project`, and runs
   `render_scene`. `render_track_mixdown` clones the project with one track
   soloed and reuses the same path.
5. Checkpoints snapshot the whole project; `rollback_session` restores the
   session-start checkpoint.

## Allowed dependencies

- `alp_core` — project model, commands, ids, `CompositeCommand`
- `alp_media` — decode, `PcmBuffer`, export
- `alp_dsp` — `measure_lufs`
- `alp_engine` — scene construction and offline render
- `alp_voice` — voice store and synthesis entry points
- `crate::{audit, checkpoint, error, jobs, sandbox, types}`

## Threading / real-time notes

The session is used from one thread at a time behind the caller's lock
(`McpServer` holds an `Arc<Mutex<ApiSession>>`). Renders are offline and
synchronous. Nothing in this module allocates or locks on the real-time audio
callback; the engine reads project state between blocks.

## Error handling

- `ApiError::PermissionDenied` for tier failures, `ApiError::NotFound` for
  unknown handles, `ApiError::Conflict` for revision mismatches,
  `ApiError::Validation` for malformed values and failed renders.
- `execute_command` converts a core command error into `ApiError::Core(Validation)`
  and records the failed attempt in the audit log.
- A failed `render_scene` is reported as `ApiError::Validation` with the engine's
  message.

## Testing

Unit tests in `session.rs` cover undo/redo, checkpoint restore, revision
tracking, permission checks and the audit log. `group_commands_since` is
exercised by the batch and script tests in `track_clip_ops.rs`,
`script_voice_ops.rs` and `crates/alp-api/tests/export_and_script.rs`.

## Known limits

- One writer per project. There is no concurrent session coordination or file
  locking between processes.
- `project_info.undo_steps`/`redo_steps` report availability (0/1), not counts.
- `verify_revision` exists but no MCP write path accepts `expected_revision`
  yet, so concurrent drift is not detected at the protocol boundary.
- Rendering re-decodes every source on every call; there is no decoded-audio
  cache.
