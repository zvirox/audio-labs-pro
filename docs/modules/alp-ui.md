# `alp-ui`

## Purpose

The shipping desktop binary. It owns one transparent window, the custom chrome,
the floating panel workspace, the command palette, keyboard shortcuts, the
synthetic waveform, a live single-track edit session, input recording/metering
and worker-backed mix export. It is the only crate that renders UI and the only
place allowed to talk to the compositor through `glass`.

## Public API

The crate is a binary, not a library. `main` registers Inter, opens an
undecorated transparent window with a 960x640 minimum and boots `app::App`.
Inside the binary:

- `app::{App, Message}` — state machine, subscriptions and update logic.
- `app::update` — the message reducer and recording/export helpers.
- `app::views` — pure screen and panel rendering.
- `session::EditSession` — live project, command stack, selection, clipboard,
  import and export bridge.
- `commands` — registry, filtering, shortcuts and palette state.
- `panels` — eight-panel geometry, z-order, snap, collapse and dock model.
- `workspace` — versioned `workspace.json` load/save.
- `glass` — compositor probe and KWin X11 blur request.
- `jobs` — cancellable background worker queue.
- `demo` — deterministic 48 kHz peaks for the shell.

## Data flow

```text
iced window/input ──► Message ──► App::update ──► panels/commands/jobs/glass
                                                    │
                                                    ├─► EditSession ──► alp-core commands
                                                    │        ├─► MixScene ──► background export
                                                    │        └─► record WAV ──► import
                                                    ├─► workspace.json (atomic)
                                                    └─► Task (focus/exit/window)

alp_media::PeakBuilder ──► WaveformPeaks ──► WaveformWidget ──► canvas
worker thread ──► mpsc ◄── JobQueue::poll during a frame ──► Export panel
```

Views read `alp-ui-kit` tokens/components and `App` state. All timeline edits go
through `EditSession` and `alp-core::CommandStack`; the views never mutate the
project. The UI starts capture through `alp-engine`, but never calls cpal
directly and never runs mixer code on the UI thread.

## Allowed dependencies

`alp-core`, `alp-engine`, `alp-media`, `alp-ui-kit`, `alp-voice`, `iced`,
`serde`, `serde_json`, `tracing` and `x11rb`. `scripts/check-deps.sh` rejects an
`alp-infer`, `alp-dsp` or cpal dependency. `alp-ui-kit` must never depend on an
ALP crate.

## Threading / real-time notes

All UI state lives on the main thread. Frames are subscribed only while a panel
moves, a recording or playback runs, a job runs, or a diagnostic env var requests
them; otherwise the app idles at zero CPU. Workers use standard threads and
report over a non-blocking channel. Recording meters are atomics sampled from a
frame; capture finalization and import can briefly block after a stop. Mix
export runs on a worker from a cloned `MixScene`.

## Error handling

Workspace save failures are visible in the Models panel and do not stop the
shell. A failed glass probe returns a tint report. A missing window id turns
window actions into no-ops. Jobs carry failure text to the queue. Recording,
import failed decode and command errors become a toast or inspector status; a
project-validation failure after a command triggers an undo. Asset or demo
invariants use `expect` only where the adjacent API guarantees them.

## Testing

`cargo test -p alp-ui` covers app shortcuts/time formatting, command filtering,
panel geometry and persistence, workspace round-trips, job lifecycle, glass mode
resolution, demo peaks and `EditSession` demo/edit/import behavior. `cargo clippy --workspace --all-targets -- -D warnings`
and `scripts/check-ui.sh` guard the crate. Manual acceptance runs use
`ALP_DEMO_SECONDS`, `ALP_DEMO_AUTOSCROLL`, `ALP_METRICS`, `ALP_AUTOCLOSE_SECS`
and `ALP_FORCE_TINT`; results are in `docs/spikes/PHASE2-RESULTS.md`.

## Known limits

- Most panels remain visual stubs; the timeline itself is still the synthetic
  waveform rather than a drawing of the live project.
- The inspector edits a real in-memory demo project, but it is not yet opened
  from or persisted to a `.alp` project directory.
- Transport playback is not connected to the engine; recording and export are.
- Import after recording runs synchronously on the UI thread.
- Real compositor blur is KWin/X11 only; Wayland and GNOME use the tint.
- No full accessibility tree, screen-reader labels or in-app text scaling yet.
- Only one window and one workspace file; monitor position is not persisted.
- `app.rs` and `alp-core::commands.rs` are above the 1,000-line soft warning;
  views and the editing session have already been split out.
