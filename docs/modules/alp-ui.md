# `alp-ui`

## Purpose

The shipping desktop binary. It owns one transparent window, the custom chrome,
the floating panel workspace, the command palette, keyboard shortcuts, the
synthetic Phase 2 waveform and the worker-backed export/import stubs. It is the
only crate that renders UI and the only place allowed to talk to the compositor
through `glass`.

## Public API

The crate is a binary, not a library. `main` registers Inter, opens an
undecorated transparent window with a 960x640 minimum and boots `app::App`.
Inside the binary:

- `app::{App, Message}` — state machine, view and subscriptions.
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
                                                    ├─► workspace.json (atomic)
                                                    └─► Task (focus/exit/window)

alp_media::PeakBuilder ──► WaveformPeaks ──► WaveformWidget ──► canvas
worker thread ──► mpsc ◄── JobQueue::poll during a frame ──► Export panel
```

The UI reads `alp-ui-kit` tokens/components and the demo peaks. It does not call
the engine callback, cpal, ONNX or GGUF directly.

## Allowed dependencies

`alp-core`, `alp-engine`, `alp-media`, `alp-ui-kit`, `alp-voice`, `iced`,
`serde`, `serde_json`, `tracing` and `x11rb`. `scripts/check-deps.sh` rejects an
`alp-infer`, `alp-dsp` or cpal dependency. `alp-ui-kit` must never depend on an
ALP crate.

## Threading / real-time notes

All UI state lives on the main thread. Frames are subscribed only while a panel
moves, the transport plays, a job runs, or a diagnostic env var requests them;
otherwise the app idles at zero CPU. Workers use standard threads and report
over a non-blocking channel. Peak generation is synchronous in this demo and is
the known startup cost. No audio callback exists in Phase 2.

## Error handling

Workspace save failures are visible in the Models panel and do not stop the
shell. A failed glass probe returns a tint report. A missing window id turns
window actions into no-ops. Jobs carry failure text to the queue. Asset or demo
invariants use `expect` only where the adjacent API guarantees them.

## Testing

`cargo test -p alp-ui` covers app shortcuts/time formatting, command filtering,
panel geometry and persistence, workspace round-trips, job lifecycle, glass mode
resolution and demo peaks. `cargo clippy --workspace --all-targets -- -D warnings`
and `scripts/check-ui.sh` guard the crate. Manual acceptance runs use
`ALP_DEMO_SECONDS`, `ALP_DEMO_AUTOSCROLL`, `ALP_METRICS`, `ALP_AUTOCLOSE_SECS`
and `ALP_FORCE_TINT`; results are in `docs/spikes/PHASE2-RESULTS.md`.

## Known limits

- The eight panels are visual stubs; the transport is not connected to
  `alp-engine` and no real project/media is opened.
- Real compositor blur is KWin/X11 only; Wayland and GNOME use the tint.
- No full accessibility tree, screen-reader labels or in-app text scaling yet.
- Only one window and one workspace file; monitor position is not persisted.
- The app binary is at the file-length ceiling; the next feature should split
  views into a separate module first.
