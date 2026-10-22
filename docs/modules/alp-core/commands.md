# `alp-core::commands`

## Purpose

Implements every Phase 1 project mutation as a reversible `Command`. Each type
stores enough data to restore the previous state, including removed objects and
their positions.

## Public API

- Sources: `AddSource`, `RemoveSource`
- Tracks: `AddTrack`, `RemoveTrack`, `MoveTrack`, `RenameTrack`, `SetTrackGain`,
  `SetTrackPan`, `SetTrackMuted`, `SetTrackSolo`
- Clips: `AddClip`, `RemoveClip`, `MoveClip`, `TrimClip`, `SplitClip`,
  `SetClipGain`, `SetClipFades`, `SetClipEnvelope`, `SetClipEffects`
- Project: `RenameProject`

Constructors take the target ids and new values. `SplitClip` is a single
command that splits one clip into a left clip and a new right clip whose id the
caller supplies, so the new clip can be selected immediately.

## Data flow

`apply` finds the target, validates the new value and writes it while saving the
old value in the command. `revert` restores that value or re-inserts a removed
object at its recorded index. Move commands clamp or validate indices before
mutating. Clip commands preserve `source_range` unless trimming explicitly.

## Allowed dependencies

`alp-core` only (`Project`, ids, `FrameRange`, `CoreError`, `Command`).

## Threading / real-time notes

Pure in-memory mutation on the owning thread. No I/O, locks or allocation on the
audio thread; allocation happens when the command is constructed.

## Error handling

Missing targets produce `TrackNotFound`, `ClipNotFound` or `SourceNotFound`.
Invalid values and out-of-range indices produce `Invalid` or
`IndexOutOfBounds`. Every command checks before mutating, so a failed apply is a
no-op.

## Testing

Unit tests cover add/remove track, split, move and rename round-trips. The
property test exercises fourteen command types. Add/remove source, rename
project, envelope and effects commands are implemented but have no direct unit
tests yet.

## Known limits

- No transaction/group command.
- Effect parameters are stored but not interpreted.
- Split does not copy every future per-clip attribute automatically; new fields
  must be added to `Clip` cloning and the split test.
