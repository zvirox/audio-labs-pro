# `alp-ui` / `session/operations`

## Purpose

Encapsulates all timeline, clip, track and marker editing operations as undoable
commands routed through the command stack.

## Public API

Extends `EditSession` with:
- Track controls: `track_count`, `tracks`, `master_gain`, `set_master_gain`, `set_track_gain`, `set_track_pan`, `toggle_track_mute`, `toggle_track_solo`, `toggle_track_arm`.
- Clip moves and crossfades: `move_selected_clip_to_next_track`, `move_selected_clip_to_prev_track`, `crossfade_selected`.
- Selection and editing: `select_clip`, `set_selection`, `copy`, `cut`, `paste`, `delete`, `silence`, `dip_selection`, `reset_envelope`, `trim`, `split`, `duplicate`, `normalize`, `toggle_reverse`, `toggle_invert`, `set_gain`, `set_fades`.
- Markers and loop range: `set_loop_range`, `clear_loop_range`, `loop_range`, `markers`, `add_marker`, `remove_marker`.
- History controls: `undo`, `redo`.

## Data flow

UI views and inspector buttons invoke these methods directly. Every method constructs
a concrete `Command` from `alp-core`, executes it on `CommandStack`, verifies the project
state, and marks the session dirty for autosave.

## Allowed dependencies

The parent `session` module, `alp-core` commands, types and math helpers.

## Threading / real-time notes

All operations run synchronously on the UI thread. Because commands mutate project
metadata in memory without file I/O or DSP rendering, operations are sub-millisecond.

## Error handling

Failed commands leave project state untouched and report descriptive error messages
in `session.status()`.

## Testing

Covered in `session/tests.rs` across clip selection, range edits, undo/redo, trims,
envelope dips, and marker management.

## Known limits

- Interactive mouse drag of clips across track boundaries on the waveform canvas is planned for Phase 5.
