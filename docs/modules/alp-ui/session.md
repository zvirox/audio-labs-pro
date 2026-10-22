# `alp-ui` / `session`

## Purpose

The live single-track editing session behind the Phase 3 inspector and
transport. It owns the project, command stack, selection, clipboard and decoded
source snapshots, and it autosaves a fixed desktop project directory.

## Public API

`EditSession` with demo construction, project/selection/clip access, selection
updates, copy/cut/paste/delete/silence/trim/split/duplicate, normalize, reverse,
invert, gain/fade updates, envelope dip/reset, background import, punch-in,
undo/redo, file import, project open/recovery/autosave, scene/render snapshots and
export settings.

## Data flow

Views send `Message`s; `App` calls session methods. Every timeline mutation
goes through `alp_core::CommandStack`. The session keeps `SourceAudio` snapshots
in a map so `MixScene::from_project` can build a render or playback scene without
touching the filesystem. Recorded WAVs are decoded and inserted with `AddSource`
plus `AddClip` commands.

## Allowed dependencies

`alp-core`, `alp-engine`, `alp-media`. No direct iced dependency.

## Threading / real-time notes

The session lives on the UI thread. Dropped-file import runs on a worker and is
polled through a non-blocking `mpsc` channel; import after a recording stop is
still synchronous. Rendering for export is handed to a `JobQueue` worker after a
scene snapshot is cloned. Autosave is debounced (750 ms) and driven by frame
ticks; close also flushes it.

## Error handling

Command failures and import/decode errors become a user-visible status line.
After every command the project is validated; a validation failure triggers an
undo of that command.

## Testing

Tests cover demo validity, selection, undoable range edits, trim, envelope dip,
copy/paste, punch-in, background import and import of a synthesized buffer.

## Known limits

- One track and one selection at a time.
- Recording import is synchronous; dropped-file import is background-decoded but
  the UI does not show per-file progress.
- The project directory is fixed to `ui-demo`; there is no picker or recent list.
- A project whose media is missing falls back to the synthetic demo instead of
  opening a repair dialog.
