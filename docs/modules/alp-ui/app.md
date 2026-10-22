# `alp-ui` — `app`

## Purpose

`app` is the shell's state machine and view layer. It owns the window id, the
floating-panel manager, theme and glass state, the command palette, the demo
waveform, the stub panel contents and the background job queue. Every mutation
enters through `Message`, so input, menu actions, shortcuts and worker results
all follow one path.

The shell is no longer a read-only demo: it opens the fixed `ui-demo` project,
the inspector edits it, recording writes real WAVs into the project, dropped
files are decoded on a worker and export renders the edited project. The timeline
drawing and transport playback are still synthetic.

## Public API

- `App::boot() -> (App, Task<Message>)` restores the workspace, builds the demo
  waveform and returns any startup task.
- `App::update(Message) -> Task<Message>` applies one message (implemented in
  `app::update`).
- `App::view() -> Element<'_, Message>` renders the full layered shell.
- `App::subscription() -> Subscription<Message>` subscribes to window events,
  raw input and animation frames. Frames are conditional.
- `App::theme() -> iced::Theme` exposes the resolved iced theme.
- `Message` is the complete shell vocabulary; the variants are documented in
  source. `Drag`, the panel item structs and the event helpers are private.

## Data flow

```text
window/input ──► Message ──► App::update ──► panels / waveform / jobs / theme
                                  │
                                  ├─► save_workspace() on layout/theme edits
                                  └─► Task (window action, palette focus, exit)

worker thread ──► mpsc channel ──► JobQueue::poll() during a frame
import worker  ──► mpsc channel ──► EditSession::poll_import() during a frame
App::view ◄── panel_manager snapshots, waveform state, palette state, theme
```

The view is rebuilt from immutable state each frame. Bitmap drawing is delegated
to `alp-ui-kit`'s waveform canvas; panels are containers with fixed geometry.

## Allowed dependencies

`alp-core` (app directories and atomic writes), `alp-engine`, `alp-media`,
`alp-voice`, `alp-ui-kit`, `iced`, `serde`, `serde_json`, `tracing` and `x11rb`
(indirectly through `glass`). The binary does not call cpal, ONNX or GGUF.

## Threading / real-time notes

The UI thread performs cheap state changes. Demo media generation and job
progress happen before the first frame or on a worker thread respectively.
`JobQueue::poll()` and `EditSession::poll_import()` drain non-blocking channels.
Frames are subscribed only while a panel animates, the transport plays, a
recording/import/job runs, or an explicit `ALP_METRICS`/
`ALP_DEMO_AUTOSCROLL` diagnostic is active, so idle CPU stays at zero. The audio
callback is never touched by the UI.

## Error handling

Workspace save errors are retained in `last_save_error` and rendered in the
Models panel instead of panicking. Jobs report failure text through the queue.
A missing window id makes window actions no-ops. Glass probing never fails the
app; it resolves to the tint fallback with a human-readable detail string.

## Testing

Unit tests cover exact shortcut mapping, time formatting and the edit session.
The panel manager, palette, job queue and workspace serializer are tested in
their own modules; 39 `alp-ui` tests pass. Manual smoke runs use `ALP_AUTOCLOSE_SECS`,
`ALP_METRICS`, `ALP_DEMO_AUTOSCROLL` and `ALP_DEMO_SECONDS`; acceptance evidence
is recorded in `docs/spikes/PHASE2-RESULTS.md`.

## Known limits

- Several panels remain stubs; transport playback and the timeline drawing are
  not connected to the engine/project yet.
- The project directory is fixed to `ui-demo`; there is no project picker, save
  as, or recent list.
- There is no in-app text-scale setting; text follows the compositor scale.
- Keyboard shortcuts and the command palette exist, but iced does not expose a
  full accessibility tree, so screen-reader support is not yet claimed.
- Floats are persisted in logical pixels; a large monitor change can leave a
  saved layout partly clamped rather than reflowed.
