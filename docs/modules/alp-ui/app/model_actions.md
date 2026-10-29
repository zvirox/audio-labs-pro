# `alp-ui` / `app` / `model_actions`

## Purpose

Neural-tier actions for the UI: resumable model downloads with inline progress, on-demand engine probes of the `alp-infer` daemon, daemon model-loading (`LoadModel`), and the tier router that lets generation surfaces prefer the daemon engine while keeping the built-in acoustic engine as an offline fallback.

## Public API

- `EngineState`: `alive`, `active_model`, `ready`; `label()` renders the one-line daemon summary
- `App::download_model(id)`: starts a background `ModelDownloader` job (gates re-checked on the worker from a fresh `ModelManager`)
- `App::load_neural_model(id)`: sends `LoadModel` through `InferClient` and records the resulting `EngineState`
- `App::check_engine()`: single `GetStatus` probe recorded into the engine state
- `App::poll_model_jobs()` / `App::drain_engine_probe()`: per-frame drains of the job queue and the engine probe slot
- `synthesize_via_tier(request, detail)`: daemon-first synthesis with built-in fallback (used by the Generate and Story renderers)

## Data flow

1. A download button spawns a `JobQueue` worker with the UI's storage dir and offline flag; the worker builds its own `ModelManager`, re-reads the persisted license markers, and runs `ModelDownloader::download` with a progress callback feeding `JobReporter`.
2. Every `Message::Frame` drains the job queue (live progress/detail per model row) and the engine probe slot; finished downloads clear the inline error and trigger one engine probe so the next action is obvious.
3. `check_engine` / `load_neural_model` probe or `LoadModel` against the default daemon socket and hand an `EngineState` back through the probe slot; failures land as inline row errors or job detail, never as silent states.
4. Generation closures call `synthesize_via_tier`: alive daemon first, builtin engine second, with the decision reported through the job's detail line.

## Allowed dependencies

- `alp-voice`: `ModelDownloader`, `ModelManager`, `ModelManagerConfig`, `ModelId`, `DownloadError`, `InferClient`, `BuiltinAcousticBackend`, `TtsBackend`, `TtsRequest`, `VoiceError`
- `alp-media` (`PcmBuffer`), `std` (`sync`, `path`), sibling `app` modules (`App`, job queue)

## Threading / real-time notes

All network and socket I/O runs on `JobQueue` worker threads; the UI thread only drains channels (`JobEvent`, engine probe slot) during `Frame` handling. No download, probe, or synthesis ever touches the audio callback or the render path. The engine probe slot mirrors the existing `generation_slot` pattern (`Arc<Mutex<Option<..>>>`).

## Error handling

Worker errors return `Err(String)` through the job queue and land next to the action that caused them: a failed download sets an inline danger-text line on its model row (both the Models panel and the Voice Lab card), a refused load reports the daemon's message as the job result, and downloads interrupted early state the exact byte boundary and the retry-to-resume contract. Gate refusals surface as toasts; nothing fails silently.

## Testing

Unit tests cover `EngineState::label` across all four daemon states (not detected, builtin tier, loaded not ready, ready) and `synthesize_via_tier` falling back to the built-in engine when no daemon listens (non-empty mono output, detail lines recorded). Protocol framing for `status` / `load_model` is pinned by `alp-voice` client tests against a fake socket daemon; download resume/progress behavior is covered by `alp-voice` downloader tests.

## Known limits

- Downloads have no cancel affordance: the downloader accepts no cancellation, so the job surface omits the button instead of offering a dead control.
- Engine state is probe-on-demand only (button, finished download, load attempt); the UI never polls the daemon in the background.
- `synthesize_via_tier` re-probes the daemon per request (a ping per take/story line), which is negligible traffic but not cached.
- The Load button is offered for every installed non-builtin TTS model; the daemon declines models without a wired engine (kitten, moonshine) with an honest message.
