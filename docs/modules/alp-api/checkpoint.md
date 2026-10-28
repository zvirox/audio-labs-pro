# `alp-api` / `checkpoint`

## Purpose

Manages project state snapshots (checkpoints) to allow single-operation rollback or complete session reversion ("Undo all AI changes").

## Public API

- `Checkpoint`: immutable snapshot of project state.
- `CheckpointManager::new() -> Self`
- `CheckpointManager::create_checkpoint(&mut self, project, label, creator) -> String`
- `CheckpointManager::restore_checkpoint(&self, id, target) -> Result<(), ApiError>`
- `CheckpointManager::initial_checkpoint_id(&self) -> Option<&str>`

## Data flow

1. When a session starts or an explicit checkpoint is requested, `create_checkpoint` deep-copies the `Project` model.
2. Checkpoint handles are stored in an in-memory index.
3. Restoring replaces the current `Project` state with the checkpoint copy.

## Allowed dependencies

- `alp_core::{Project, now_ms}`
- `serde::{Deserialize, Serialize}`
- `crate::error::ApiError`

## Threading / real-time notes

Snapshot creation involves cloning in-memory data structures on the automation thread.

## Error handling

Returns `ApiError::NotFound` if an invalid checkpoint identifier is requested.

## Testing

Covered by unit tests verifying snapshot creation, project mutation, and faithful restoration.

## Known limits

- Checkpoints exist in memory during the active session.
