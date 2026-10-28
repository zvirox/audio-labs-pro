# `alp-ui` / `app/views`

## Purpose

Pure screen and panel rendering for the shell. Views read `App` state and return
iced elements; they never mutate project or panel state.

## Public API

- `App::view` composes the shell: the rounded frame and backdrop, palette,
  toast, shortcuts, Create Voice, consent and first-run layers.
- `chrome_view`, `grid_view` and `player_view` draw the top bar, workspace body
  and the docked player shared by every workspace.
- `timeline_view`, `mixer_view`, `transport_view`, `inspector_view` provide the
  editing bodies used by the Stories workspace and the panel stack.
- `effects_view` (cleanup chain with compact switches) and `plugin_list_view`
  (hosted plugin rows) are the bodies embedded by `app::effects_views`.
- `export_view` is the compact job queue embedded by `app::settings_views`.
- Workspace screens themselves live in the sibling `*_views` modules.

## Data flow

`App::view` builds an 18 px rounded frame with a 34 px chrome strip, then a row
of 80 px rail and workspace column, with the dock under the workspace. Workspace
bodies are selected by `WorkflowTab`; the dock switches between the player and
the Stories track editor. Panel bodies are selected by `PanelKind`. Messages are
returned from controls and handled only by `App::update`.

`transport_view` keeps playback, position, gain and progress; recording and
dictation controls moved to the Captures workspace, and ripple delete lives in
the mixer.

## Allowed dependencies

The parent `app` module, `alp-ui-kit` components and tokens, and iced widgets.

## Threading / real-time notes

Rendering runs on the UI thread. Meter values and job progress are plain values
sampled from atomics/channels; views never wait on workers.

## Error handling

Views display session status, job failures, model/profile availability and the
last export path. Missing selections simply disable or reject edit actions in the
session, and missing stores render an explicit “unavailable” line.

## Testing

Rendering itself is not snapshot-tested. Model tests cover command key mapping,
time formatting, workspace slugs and studio label helpers; the workspace UI
guard rejects raw iced buttons in screen code.

## Known limits

- The timeline still draws the synthetic demo waveform.
- The palette panel is a fixed 620 px wide card with a 324 px scrolling list.
- The inspector is intentionally compact and scrolls on small windows.
- No screen-reader tree or text-scale controls yet.
