# `alp-api` / `project_ops`

## Purpose

Manages the lifecycle of Audio Labs Pro projects (listing, creating, opening, closing) and exposes undo/redo command history inspection for `ApiSession`.

## Public API

- `ProjectListItem`: project name, filesystem path, sample rate, track count.
- `HistoryEntry`: step index, action description, whether step is undoable.
- `ApiSession::project_list(&self) -> Result<Vec<ProjectListItem>, ApiError>`
- `ApiSession::project_create(&mut self, name: &str, sample_rate: Option<u32>) -> Result<String, ApiError>`
- `ApiSession::project_open(&mut self, path: &str) -> Result<String, ApiError>`
- `ApiSession::project_close(&mut self) -> Result<(), ApiError>`
- `ApiSession::history_list(&self) -> Result<Vec<HistoryEntry>, ApiError>`

## Data flow

1. Checks permissions (`PermissionTier::Observe` for queries, `PermissionTier::Edit` for project lifecycle mutations).
2. For `project_open` / `project_create`:
   - Validates destination path against the `WorkspaceSandbox`.
   - Initializes or loads `Project` schema JSON.
   - Updates active `ApiSession.project` and `ApiSession.paths`.
   - Appends audit trail entries to the persistent log.
3. For `history_list`: queries session command stack and returns ordered step descriptions.

## Allowed dependencies

- `serde::{Deserialize, Serialize}`
- `alp_core::{Project, ProjectPaths}`
- `crate::error::ApiError`
- `crate::session::ApiSession`
- `crate::types::PermissionTier`

## Threading / real-time notes

Project lifecycle mutations swap session project state under session write lock; cannot be executed concurrently with active real-time playback.

## Error handling

Returns `ApiError::Validation` on invalid project JSON or unsupported sample rates; `ApiError::SandboxEscape` if paths violate sandbox boundaries.

## Testing

Covered by project creation, open/close, and history listing integration tests.

## Known limits

- Only one active project is loaded in a session at a time.
