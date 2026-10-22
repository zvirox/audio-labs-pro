# `alp-ui` / `app/update`

## Purpose

The shell's message reducer. Every user action, window event, frame tick, job
result and edit command is handled here; views never mutate state directly.

## Public API

`App::update(&mut self, Message) -> Task<Message>` plus private helpers for
selection synchronization, recording/punch start-stop, background import polling,
mix export, command execution, keyboard handling, window sizing, panel visibility,
workspace save and toasts.

## Data flow

Messages arrive from iced or from view controls. The reducer updates panel,
workspace, job or `EditSession` state and returns an iced `Task` only for window,
focus or exit effects. Editing messages call `EditSession`, which routes
mutations through `alp_core::CommandStack`. Export clones a `MixScene` and hands
it to `JobQueue`; recording starts `alp_engine::Recording` and samples its atomics
on frames. Dropped files start a background decode and are inserted when
`EditSession::poll_import()` completes. Punch-in replaces the selected range
instead of appending the take.

## Allowed dependencies

The parent `app` module, `alp-core`, `alp-engine`, `alp-media`, `alp-ui-kit`,
`crate::jobs`, `crate::session`, `crate::panels` and iced tasks/events.

## Threading / real-time notes

Runs on the UI thread. It never calls the mixer callback or waits on a worker.
Recording stop and import-after-record can block briefly; dropped-file import
and background export do not.

## Error handling

Errors from capture start/stop, import, export setup and command validation are
shown as toasts or inspector status. A failed command never leaves an invalid
project: `EditSession` validates after each command and undoes on failure.

## Testing

Reducer behavior is covered indirectly by `alp-ui` model tests (commands,
workspace, jobs, glass) and by `EditSession` tests. Hardware capture and export
paths remain manual/CLI acceptance.

## Known limits

- Some existing panels still carry demo-only messages.
- Workspace save happens on panel changes. Project autosave is debounced to
  750 ms and flushed on close; a hard kill can lose the last debounce window.
- The reducer samples recording meters; it does not draw a live input waveform.
