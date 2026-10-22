# `alp-core::command`

## Purpose

Defines the mutation protocol and the bounded undo/redo history. Commands are
self-contained descriptions of one edit; the stack owns them but never mutates
the project itself.

## Public API

- `Command` trait: `label`, `apply`, `revert`
- `CommandError` variants for missing tracks/clips/sources, bad indices,
  duplicates, invalid state and wrapped `CoreError`
- `CommandStack::new`, `with_limit`, `execute`, `undo`, `redo`, `can_undo`,
  `can_redo`, `undo_len`, `redo_len`, `undo_label`, `redo_label`, `clear`
- `CommandStack::DEFAULT_LIMIT` (512)

## Data flow

`execute` calls `apply`, clears the redo stack, pushes the command and trims the
oldest entries past the limit. `undo` reverts the newest command and moves it to
the redo stack. `redo` re-applies it and trims the undo stack. A failed command
is not pushed.

## Allowed dependencies

Only other `alp-core` modules and `thiserror`. No ALP crate dependencies.

## Threading / real-time notes

The stack is owned by the UI thread and is not internally synchronised. It must
never be used from the audio callback; the engine receives a prepared scene
snapshot instead.

## Error handling

Commands must leave the project untouched when they return an error. The stack
propagates the error and keeps history unchanged. `InconsistentState` reports a
command whose revert does not match its apply.

## Testing

Unit tests cover execute/undo/redo ordering, redo invalidation and history
limits. The integration property test applies a command sequence, undoes it
completely and redoes it, comparing serialised projects.

## Known limits

- History is in-memory only; it is not saved with the project.
- There is no grouping/transaction command for multi-clip edits yet.
