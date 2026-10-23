# # `alp-core::commands::track`

## Purpose

Undoable whole-track commands: add, remove, rename, gain, pan, mute, solo, colour, arm, and display-order moves.

## Public API

Undoable whole-track commands: add, remove, rename, gain, pan, mute, solo, colour, arm, and display-order moves.

## Data flow

Tracks are addressed by [`TrackId`]. Each command records the previous value or index so `revert` restores the exact pre-edit project. `SetTrackArm` and `SetTrackColor` follow the same pattern introduced for mute/solo.

## Allowed dependencies

`alp-core` only.

## Threading / real-time notes

Pure in-memory data mutation; no audio-thread use.

## Error handling

Missing targets report `TrackNotFound`; out-of-range indices report `IndexOutOfBounds`. Validation happens before mutation.

## Testing

Added through the command stack alongside the existing track tests in `commands::tests`.

## Known limits

Track recording arm exists in the data model only; live arming, metering and multi-input recording still need engine and UI work.
