# `alp-ui` / `app/studio`

## Purpose

Owns the voice-studio operations behind the seven workspaces: profile CRUD,
background speech rendering, generated-take history, preview playback, story
rendering, capture bookkeeping, transcript export and model policy.

## Public API

- `App::profile` / `profile_name` / `select_profile` / `reload_profiles`
- `App::start_generation` / `poll_generation` / `play_buffer` / `play_file`
- `App::play_history` / `insert_history` / `delete_history`
- `App::create_designed_profile` / `create_cloned_profile` / `delete_profile`
- `App::export_profile` / `import_profile`
- `App::story_add_line` / `story_delete_line` / `story_new` / `story_save`
- `App::story_render` / `record_capture` / `transcribe_capture`
- `App::audit_reference` / `export_transcript`
- `App::accept_model_license` / `remove_model` / `toggle_offline_models`

## Data flow

1. Generation snapshots the prompt and profile, reserves a WAV path under the
   data directory, and spawns a `JobQueue` worker.
2. The worker synthesizes with `BuiltinAcousticBackend`, writes a 24-bit WAV with
   `alp_media::export`, and stores a `RenderOutput` in the shared slot.
3. `Message::Frame` drains the slot, appends a history item, updates story line
   audio, and reports through the toast/status line.
4. History playback builds a one-clip `MixScene`; insertion decodes the take and
   calls `EditSession::import_buffer`, so the edit is an undoable command.
5. `transcribe_capture` is daemon-first (ADR 0025): it decodes the capture,
   asks `InferClient` to transcribe when the daemon listens (naming the
   engine from the same daemon's `active_stt_model` in the toast), and falls
   back to the in-process `BuiltinSttBackend` otherwise. The
   placeholder-words admission stays whenever the acoustic segmenter - on
   either side - did the transcribing.

## Allowed dependencies

- `alp-core`, `alp-media`, `alp-engine` (preview playback), `alp-voice`
- `iced` message plumbing only; no widget construction
- `std::sync::{Arc, Mutex}` for the worker hand-off

## Threading / real-time notes

Every synthesis, story render and disk export runs on a worker thread. The UI
thread never blocks on the engine; the slot is a mutex-protected one-slot
mailbox drained once per frame. Playback always crosses `alp-engine`, never a
private audio thread.

## Error handling

Worker failures are returned as `Err(String)` through the slot and surfaced in
the status line plus a toast. Missing profiles, missing references, empty
prompts and consent violations are rejected before spawning work. Disk and
decode errors are reported without mutating project state.

## Testing

Covered indirectly by `cargo test --workspace` and the UI smoke run. Profile,
story and render paths are deterministic with `ALP_DATA_HOME` redirected.

## Known limits

- `BuiltinAcousticBackend` is a formant synthesizer with no timbre cloning.
- Story rendering uses the built-in engine only and assumes 48 kHz mono for
  stitching.
- Generated history is not persisted across launches.
