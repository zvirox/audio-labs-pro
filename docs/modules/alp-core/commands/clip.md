# # `alp-core::commands::clip`

## Purpose

Undoable clip commands: add, remove, move, trim, split, gain, fades, envelope and effects assignment.

## Public API

Undoable clip commands: add, remove, move, trim, split, gain, fades, envelope and effects assignment.

## Data flow

Clip edits use shared `clip_location` plus command-owned snapshots (`removed`, `previous`) so clip ids stay stable across undo and redo.

## Allowed dependencies

`alp-core` only.

## Threading / real-time notes

No audio-thread use.

## Error handling

Missing targets report `ClipNotFound`; out-of-range indices and empty clips are rejected before mutation.

## Testing

Unit tests cover split, move, and rename round-trips; more branches still need property coverage.

## Known limits

Crossfades and ripple semantics across tracks are not represented as distinct commands yet.
