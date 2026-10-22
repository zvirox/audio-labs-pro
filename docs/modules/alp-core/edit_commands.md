# `alp-core` / `edit_commands`

## Purpose

Undoable Phase 3 edits over one track: range delete/silence, paste, duplicate,
punch-in, normalize, reverse and invert.

## Public API

- `DeleteRange::new(track_id, range, ripple)`
- `SilenceRange::new(track_id, range)`
- `PasteClips::new(track_id, at, fragments)`
- `DuplicateClip::new(clip_id, new_id, offset)`
- `PunchIn::new(track_id, range, fragments)`
- `NormalizeClip::new(clip_id, target_peak)`
- `SetClipReversed::new(clip_id, bool)`
- `SetClipInverted::new(clip_id, bool)`

All implement `alp_core::Command`.

## Data flow

Range commands compute the complete replacement clip list once, cache it, and
restore it on redo. Split ids are allocated once and therefore stay stable over
undo/redo. `DeleteRange { ripple: true }` closes the gap; `SilenceRange` keeps
later clips in place. `PunchIn` replaces the range with recorded fragments in a
single history entry.

## Allowed dependencies

Only `alp-core` modules: project data, ids, time, clipboard helpers and the
command protocol.

## Threading / real-time notes

Commands mutate owned project data on the UI/worker thread. No command touches
the audio callback; the engine consumes an immutable scene snapshot.

## Error handling

Empty/inverted ranges, missing tracks or clips, duplicate ids, fragments that
read past a source, fades longer than the clip, and normalization without a
measured source peak are rejected with `CommandError`.

## Testing

Inline tests cover split/ripple behaviour, silence-in-place, paste offsets,
duplicate count, reverse/invert/normalize undo, punch-in and missing-peak
rejection. `tests/undo_redo.rs` includes all Phase 3 command families.

## Known limits

- Normalization uses the source-level peak, not a peak measured over the clip
  region; trim a quieter section and normalize separately if needed.
- Punch-in accepts prepared fragments; capture and UI import live in
  `alp-engine`/`alp-ui`.
