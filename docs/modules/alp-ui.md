# `alp-ui`

## Purpose

The shipping desktop binary. It owns one transparent window, the custom chrome,
the seven-workspace shell, the glyph navigation rail, the docked player, the
command palette, keyboard shortcuts, a live single-track edit session, input
recording/metering, voice and story generation, capture transcription and
worker-backed mix export. It is the only crate that renders UI and the only
place allowed to talk to the compositor through `glass`.

The shell uses a retro palette from `alp-ui-kit`: a dark “midnight walnut”
theme (espresso surfaces, cream text, burnt-orange accent, teal secondary) and a
light “aged paper” variant, with 3/7/11 radii and soft card shadows.

## Public API

The crate is a binary, not a library. `main` registers Inter, opens an
undecorated transparent window with a 960x640 minimum and boots `app::App`.
Inside the binary:

- `app::{App, Message}` — state machine, subscriptions and update logic.
- `app::update` — the message reducer and recording/export helpers.
- `app::views` — pure screen and panel rendering.
- `app::workspace_layout` — `WorkflowTab`, the glyph rail, docked player,
  `card`/`card_auto`, `header` and `metric_badge`.
- `app::studio` — profile CRUD/import/export, generation, story and capture
  operations against `alp-voice`.
- `app::generate_views`, `app::stories_views`, `app::capture_views`,
  `app::voice_views`, `app::effects_views`, `app::settings_views`,
  `app::ai_views`, `app::wizard_views` — one workspace or overlay each.
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
                                                    ├─► ProfileStore ──► .alpvoice bundles
                                                    ├─► Script (JSON) ──► rendered story
                                                    ├─► ModelManager ──► licence + storage
                                                    ├─► workspace.json (atomic)
                                                    └─► Task (focus/exit/window)

alp_media::PeakBuilder ──► WaveformPeaks ──► WaveformWidget ──► canvas
worker thread ──► mpsc ◄── JobQueue::poll during a frame ──► Export panel
generation thread ──► Arc<Mutex<Option<Result<RenderOutput, String>>>>
                     ◄── drained in Message::Frame ──► history row
```

Views read `alp-ui-kit` tokens/components and `App` state. All timeline edits go
through `EditSession` and `alp-core::CommandStack`; the views never mutate the
project. Generated audio enters the project as an undoable
`EditSession::import_buffer`. The UI starts capture through `alp-engine`, but
never calls cpal directly and never runs mixer code on the UI thread.

## Allowed dependencies

`alp-api`, `alp-core`, `alp-engine`, `alp-media`, `alp-plugin`, `alp-ui-kit`,
`alp-voice`, `iced`, `serde`, `serde_json`, `tracing` and `x11rb`.
`scripts/check-deps.sh` rejects an `alp-infer`, `alp-dsp` or cpal dependency, so
effect DSP that the UI must call has to route through `alp-api`. `alp-ui-kit`
must never depend on an ALP crate.

## Threading / real-time notes

All UI state lives on the main thread. Frames are subscribed only while a panel
moves, a recording or playback runs, a job runs, or a diagnostic env var requests
them; otherwise the app idles at zero CPU. Workers use standard threads and
report over a non-blocking channel. Recording meters are atomics sampled from a
frame; capture finalization and import can briefly block after a stop. Mix
export runs on a worker from a cloned `MixScene`; voice generation and story
rendering run on their own thread and publish one result that
`Message::Frame` drains.

## Error handling

Workspace save failures are visible in the Settings workspace and do not stop the
shell. A failed glass probe returns a tint report. A missing window id turns
window actions into no-ops. Jobs carry failure text to the queue. Recording,
import failed decode, generation and command errors become a toast, a workspace
status line or inspector status; a project-validation failure after a command
triggers an undo. Missing profile stores, model folders or story files render an
explicit “unavailable” state rather than failing boot. Asset or demo invariants
use `expect` only where the adjacent API guarantees them.

## Testing

`cargo test -p alp-ui` runs 48 tests covering app shortcuts/time formatting,
command filtering, panel geometry and persistence, workspace round-trips, job
lifecycle, glass mode resolution, demo peaks, `EditSession` demo/edit/import
behavior, the `WorkflowTab` slug round-trip used by `ALP_START_TAB`, and the
studio label helpers (`slugify`, `clock_label`). `cargo clippy --workspace --all-targets -- -D warnings`
and `scripts/check-ui.sh` guard the crate. Manual acceptance runs use
`ALP_DEMO_SECONDS`, `ALP_DEMO_AUTOSCROLL`, `ALP_METRICS`, `ALP_AUTOCLOSE_SECS`,
`ALP_START_TAB` and `ALP_FORCE_TINT`; results are in `docs/spikes/PHASE2-RESULTS.md`.

## Known limits

- The timeline still draws the synthetic demo waveform rather than the live
  project; the inspector and the docked player operate on real data.
- The inspector edits a real in-memory demo project, but it is not yet opened
  from or persisted to a `.alp` project directory.
- Built-in TTS ignores `VoiceParams` pitch, energy and emotion; cloning is a
  trait-only seam, so the “Cloned” kind records consent but does not match a
  reference timbre.
- Built-in STT writes placeholder segment text; transcription timing and export
  work, the words are a stand-in.
- The Effects workspace prepares the cleanup chain, but `alp-engine` does not
  apply `EffectRef` yet, so playback is unprocessed.
- There is no HTTP model downloader; models must be placed and verified by hand.
- Import after recording runs synchronously on the UI thread.
- Real compositor blur is KWin/X11 only; Wayland and GNOME use the tint.
- No full accessibility tree, screen-reader labels or in-app text scaling yet.
- Only one window and one workspace file; monitor position is not persisted.
- `app/update.rs` is above the 1,000-line soft warning; views, studio and the
  editing session have already been split out.
