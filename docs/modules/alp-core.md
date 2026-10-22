# `alp-core`

## Purpose

Owns the project model, typed identifiers, audio-time helpers, the undo/redo
protocol and atomic persistence. It is the root of the ALP crate graph and has
no dependency on any other workspace crate, so the domain can be tested without
audio devices, codecs or a UI.

## Public API

- `Project`, `Source`, `Track`, `Clip`, `EnvelopePoint`, `EffectRef`
- `Project::duration_frames`, `clip_timeline_len`, `clip_timeline_end`,
  `validate`
- `ProjectId`, `SourceId`, `TrackId`, `ClipId`
- `FrameRange`, `secs_to_frames`, `frames_to_secs`
- `Command`, `CommandError`, `CommandStack`
- Concrete commands: `AddSource`, `AddTrack`, `AddClip`, `RemoveSource`,
  `RemoveTrack`, `RemoveClip`, `MoveTrack`, `MoveClip`, `TrimClip`, `SplitClip`,
  `SetClipGain`, `SetClipFades`, `SetClipEnvelope`, `SetClipEffects`,
  `SetTrackGain`, `SetTrackPan`, `SetTrackMuted`, `SetTrackSolo`,
  `RenameProject`, `RenameTrack`
- `ProjectPaths`, `AppDirs`, `to_json`, `from_json`, `schema_version`
- `write_atomic`, `create_temp_sibling`, `commit_temp`
- `init_logging`, `CoreError`

## Data flow

1. A caller loads a project with `ProjectPaths::open` or creates one with
   `ProjectPaths::create`.
2. Mutations go through `CommandStack::execute`, which calls `Command::apply`
   and pushes the command onto the undo stack (clearing redo).
3. `CommandStack::undo` calls `revert`; `redo` applies again.
4. `ProjectPaths::save` serialises the project to the versioned JSON envelope,
   writes a sibling temp file, fsyncs it and renames it into place.
5. Media paths are stored relative to the project root and resolved only at
   load or render time.

## Allowed dependencies

None. `alp-core` must not depend on any other `alp-*` crate. External crates are
`serde`, `serde_json`, `uuid`, `dirs`, `thiserror`, `tracing` and
`tracing-subscriber`.

## Threading / real-time notes

All types are plain data and are not shared with the audio callback. The engine
takes an owned snapshot instead. `CommandStack` is `Send` but not internally
synchronised; the UI owns it on one thread. Atomic file writes may block and
must never run on the audio thread.

## Error handling

`CoreError` distinguishes validation, not-found, duplicate, index, I/O and
schema-version failures. Commands return `CommandError`, which preserves the
failed command so a caller can retry. Loading rejects unknown future schema
versions instead of guessing. `Project::validate` checks structural invariants
before every save.

## Testing

Unit tests cover IDs, frame conversion, project validation, atomic files and
path helpers, plus apply/revert for the most-used commands. The
`tests/undo_redo.rs` property test applies random sequences across fourteen
command types, then checks that full undo and redo reproduce the serialised
project byte-for-byte.

## Known limits

- Version 1 has one schema migration scaffold and no historical migrations.
- `EffectRef` stores parameters as JSON, but effect rendering lands in Phase 5.
- Project files are JSON; binary sidecars are not part of the format yet.
