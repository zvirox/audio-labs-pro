# `alp-ui` / `app/views`

## Purpose

Pure screen and panel rendering for the shell. Views read `App` state and return
iced elements; they never mutate project or panel state.

## Public API

`App::view` plus private panel views for chrome, transport, timeline, inspector,
media bin, voice lab, effects, models and export queue.

## Data flow

`App::view` builds the base tint, one layer per visible panel, chrome, palette,
toast and modal. Panel bodies are selected by `PanelKind`. Messages are returned
from controls and handled only by `App::update`.

## Allowed dependencies

The parent `app` module, `alp-ui-kit` components and tokens, and iced widgets.

## Threading / real-time notes

Rendering runs on the UI thread. Meter values and job progress are plain values
sampled from atomics/channels; views never wait on workers.

## Error handling

Views display session status, job failures and the last export path. Missing
selections simply disable or reject edit actions in the session.

## Testing

Rendering itself is not snapshot-tested. Model tests cover command key mapping
and time formatting; the workspace UI guard rejects raw iced buttons in screen
code.

## Known limits

- The inspector is intentionally compact and scrolls on small windows.
- No screen-reader tree or text-scale controls yet.
