# `alp-ui` / `app/update`

## Purpose

The shell's message reducer. Every user action, window event, frame tick, job
result and edit command is handled here; views never mutate state directly.

## Public API

`App::update(&mut self, Message) -> Task<Message>` plus private helpers for
selection synchronization, recording/punch start-stop, background import polling,
mix export, command execution, keyboard handling, window sizing, panel visibility,
workspace save and toasts. Workspace-specific handlers live in `app/studio`, but
they are all reached from this reducer.

## Data flow

Messages arrive from iced or from view controls. The reducer updates panel,
workspace, job or `EditSession` state and returns an iced `Task` only for window,
focus or exit effects. Editing messages call `EditSession`, which routes
mutations through `alp_core::CommandStack`. Export clones a `MixScene` and hands
it to `JobQueue`; recording starts `alp_engine::Recording` and samples its atomics
on frames. Dropped files start a background decode and are inserted when
`EditSession::poll_import()` completes; a dropped `.alpvoice` bundle is imported
through the profile store instead. Punch-in replaces the selected range instead
of appending the take.

`Message::SelectWorkflowTab` switches the seven-workspace shell. Generate,
profile, story, capture and model messages delegate to `app::studio`, which owns
the `ProfileStore`, `Script` files and `ModelManager`. Voice generation and story
rendering run on their own thread; `Message::Frame` drains the result slot,
updates history and clears the busy flag. Capture stop also appends a row to the
Captures workspace and replaces the transcript preview with the VAD result.

## Allowed dependencies

The parent `app` module, `alp-api`, `alp-core`, `alp-engine`, `alp-media`,
`alp-plugin`, `alp-ui-kit`, `alp-voice`, `crate::jobs`, `crate::session`,
`crate::panels` and iced tasks/events.

## Threading / real-time notes

Runs on the UI thread. It never calls the mixer callback or waits on a worker.
Recording stop and import-after-record can block briefly; dropped-file import,
background export and voice/story rendering do not.

## Error handling

Errors from capture start/stop, import, export setup, generation, profile or
story IO and command validation are shown as toasts or workspace status lines. A
failed command never leaves an invalid project: `EditSession` validates after
each command and undoes on failure. A missing profile store or model folder
leaves the workspace in an explicit unavailable state instead of panicking.

## Testing

Reducer behavior is covered indirectly by `alp-ui` model tests (commands,
workspace, jobs, glass, studio labels) and by `EditSession` tests. Hardware
capture and export paths remain manual/CLI acceptance.

## Known limits

- Some legacy panel messages are retained only for the palette and AI clients.
- Workspace save happens on panel changes. Project autosave is debounced to
  750 ms and flushed on close; a hard kill can lose the last debounce window.
- The reducer samples recording meters; it does not draw a live input waveform.
- The file is above the 1,000-line soft warning; further workspace groups should
  move to `app/studio` or a new sibling module.
