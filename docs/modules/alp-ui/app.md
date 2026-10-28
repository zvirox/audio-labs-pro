# `alp-ui` — `app`

## Purpose

`app` is the shell's state machine and view layer. It owns the window id, the
floating-panel manager, the seven-workspace selection, the docked player, theme
and glass state, the command palette, the demo waveform, the background job
queue, the voice profile store, story files and the model manager. Every
mutation enters through `Message`, so input, menu actions, shortcuts and worker
results all follow one path.

The shell is no longer a read-only demo: it opens the fixed `ui-demo` project,
the inspector edits it, recording writes real WAVs into the project, dropped
files are decoded on a worker and export renders the edited project. Voice
generation, profiles, stories, captures and models are real operations on
`alp-voice` state. The timeline drawing and transport playback are still
synthetic.

## Public API

- `App::boot() -> (App, Task<Message>)` restores the workspace, opens the profile
  store, story and model directories, builds the demo waveform and honours
  `ALP_START_TAB` for smoke runs.
- `App::update(Message) -> Task<Message>` applies one message (implemented in
  `app::update`).
- `App::view() -> Element<'_, Message>` renders the full layered shell.
- `App::subscription() -> Subscription<Message>` subscribes to window events,
  raw input and animation frames. Frames are conditional.
- `App::theme() -> iced::Theme` exposes the resolved iced theme.
- `Message` is the complete shell vocabulary; the variants are documented in
  source. `Drag`, the panel item structs and the event helpers are private.
- Sibling modules own the workspaces: `app::workspace_layout` (tabs, rail,
  cards), `app::studio` (profile/generation/story/capture/model operations) and
  one `*_views` module per workspace.

## Data flow

```text
window/input ──► Message ──► App::update ──► panels / waveform / jobs / theme
                                   │
                                   ├─► save_workspace() on layout/theme edits
                                   ├─► studio ──► ProfileStore / Script / ModelManager
                                   └─► Task (window action, palette focus, exit)

worker thread ──► mpsc channel ──► JobQueue::poll() during a frame
import worker  ──► mpsc channel ──► EditSession::poll_import() during a frame
generation thread ──► Arc<Mutex<Option<Result<RenderOutput, String>>>>
                      ◄── Frame ──► history row / status
App::view ◄── panel_manager snapshots, waveform state, palette state, theme
```

The view is rebuilt from immutable state each frame. Bitmap drawing is delegated
to `alp-ui-kit`'s waveform canvas; workspaces are columns of retro cards. A
generated take is inserted through `EditSession::import_buffer`, so it stays
undoable like any other edit.

## Allowed dependencies

`alp-api`, `alp-core` (app directories and atomic writes), `alp-engine`,
`alp-media`, `alp-plugin`, `alp-voice`, `alp-ui-kit`, `iced`, `serde`,
`serde_json`, `tracing` and `x11rb` (indirectly through `glass`). The binary does
not call cpal, ONNX or GGUF, and cannot link `alp-dsp`.

## Threading / real-time notes

The UI thread performs cheap state changes. Demo media generation and job
progress happen before the first frame or on a worker thread respectively.
`JobQueue::poll()` and `EditSession::poll_import()` drain non-blocking channels,
and `Message::Frame` drains the generation slot. Frames are subscribed only while
a panel animates, the transport plays, a recording/import/job runs, a generation
is pending, or an explicit `ALP_METRICS`/`ALP_DEMO_AUTOSCROLL` diagnostic is
active, so idle CPU stays at zero. The audio callback is never touched by the UI.

## Error handling

Workspace save errors are retained in `last_save_error` and rendered in the
Settings workspace instead of panicking. Jobs report failure text through the
queue, and generation failures land in the same toast/status path. A missing
window id makes window actions no-ops. Glass probing never fails the app; it
resolves to the tint fallback with a human-readable detail string. A missing
profile store, story directory or model folder degrades to an explicit
“unavailable” line.

## Testing

Unit tests cover exact shortcut mapping, time formatting, the edit session, the
`WorkflowTab` slug round-trip and the studio label helpers. The panel manager,
palette, job queue, glass mode and workspace serializer are tested in their own
modules; 48 `alp-ui` tests pass. Manual smoke runs use `ALP_AUTOCLOSE_SECS`,
`ALP_METRICS`, `ALP_DEMO_AUTOSCROLL`, `ALP_DEMO_SECONDS`, `ALP_START_TAB` and
`ALP_FORCE_TINT`; acceptance evidence is recorded in `docs/spikes/PHASE2-RESULTS.md`.

## Known limits

- Several legacy panels remain stubs; transport playback and the timeline drawing
  are not connected to the engine/project yet.
- The project directory is fixed to `ui-demo`; there is no project picker, save
  as, or recent list.
- Built-in TTS ignores pitch/energy/emotion, cloning is a trait-only seam and STT
  emits placeholder segment text; the UI states this where it matters.
- Effects are prepared in the UI but not applied by the engine yet.
- There is no in-app text-scale setting; text follows the compositor scale.
- Keyboard shortcuts and the command palette exist, but iced does not expose a
  full accessibility tree, so screen-reader support is not yet claimed.
- Floats are persisted in logical pixels; a large monitor change can leave a
  saved layout partly clamped rather than reflowed.
- `app/update.rs` is above the 1,000-line soft warning.
