# `alp-api` / `voice`

## Purpose

Voice tool methods on `ApiSession` backing the Phase 10 MCP toolset: voice listing and inspection, speech synthesis into the project media directory, voice
design, and sandboxed transcription. Engines run daemon-first (the `alp-infer` worker via `InferClient`), falling back to the in-process built-in acoustic or
segmenter backends; every result names the engine that actually ran.

## Public API

- `VoiceSummary`, `TtsVoicesReport`: one voice and the `tts_voices` result (voices list plus daemon liveness and active synthesis model).
- `VoiceParamsDetail`, `VoiceProfileDetail`: public view of a profile; `cached_embedding` and consent contents are never exposed (consent is a boolean). `VoiceProfileDetail.locked` carries the profile's lock state.
- `VoiceParamsUpdate` (alpha.6): optional per-field `{ speed?, pitch?, energy?, emotion? }` mutations.
- `VoiceProfileUpdate` (alpha.6): optional `{ name?, tags?, default_params? }` mutations of a stored profile; kind, backend and consent records are not mutable here.
- `TtsGenerationReport`: `path`, `duration_secs`, `engine`, `sample_rate` for `tts_generate`.
- `BatchTtsRequest` (alpha.6): one item of `tts_batch` — `{ voice_id, text, speed? }`.
- `BatchTtsReport` (alpha.6): `tts_batch` result — one `TtsGenerationReport` per submitted job, in request order.
- `TranscriptionSegmentReport`, `TranscriptionReport`: segments in seconds, plain text, engine name, optional written transcript path for `transcribe`.
- `ApiSession::with_voice_store_dir(dir)`: override the profile store directory (tests and embedding hosts); unset keeps the default XDG store.
- `ApiSession::tts_voices()` (Observe), `ApiSession::voice_profile_get(id)` (Observe).
- `ApiSession::tts_generate(text, voice_id, speed)` (Edit), `ApiSession::voice_design(name, prompt, tags)` (Edit).
- `ApiSession::voice_profile_update(id, update)` (Edit), `ApiSession::voice_profile_lock(id)` (Edit), `ApiSession::voice_profile_unlock(id)` (Edit).
- `ApiSession::tts_batch(&[BatchTtsRequest])` (Edit): one job per request, sequential, cancellable via `job_cancel`.
- `ApiSession::transcribe(path, format, out)` (Observe).

## Data flow

1. Calls validate the session tier via `session.check_permission`, matching the tool-tier table (list/get/transcribe are Observe; generate/design are Edit).
2. Voice metadata comes from `ProfileStore` (override directory or XDG default); the built-in seed voices are created on first open.
3. `tts_generate` validates the voice id against the store, synthesizes (daemon first, built-in fallback), writes a 16-bit WAV into the project `media/`
   directory through `WorkspaceSandbox::validate_path`, and audits the call.
4. `transcribe` decodes the sandboxed input, runs daemon-first transcription, returns segments plus plain text, and optionally writes `text`/`srt`/`vtt`
   through the sandbox.
6. `transcribe` decodes the sandboxed input, runs daemon-first transcription, returns segments plus plain text, and optionally writes `text`/`srt`/`vtt`
   through the sandbox.
7. `voice_profile_update` loads the profile from `ProfileStore`, applies the given `name`/`tags`/`default_params` changes, and saves it back
   (`merge_params` merges partial parameter updates over stored values). Locked profiles refuse changes until unlocked; `kind`, `backend` and consent
   records are never touched by this path, so cloning is never creatable through it.
8. `tts_batch` submits one `JobManager` job per request and runs them sequentially through `tts_generate_inner` (with a `is_cancelled` check between
   requests). The first error marks the failing job failed and returns `Err`; the other jobs stay queryable via `job_status`.
9. Errors and fallbacks are logged with `tracing`; results carry the honest engine name (`daemon:<model>`, `daemon:builtin-acoustic`, `daemon:builtin`,
   `builtin-acoustic`, `builtin`).

## Allowed dependencies

- `alp-core` (`frames_to_secs`, `now_ms`, project types via `crate::session`)
- `alp-media` (`decode`, `export`, `ExportSettings`, `PcmBuffer`)
- `alp-voice` (`InferClient`, `BuiltinAcousticBackend`, `BuiltinSttBackend`, `ProfileStore`, `design_voice`, transcript types)
- `serde`/`serde_json` (report serialization), `tracing`, `thiserror` via `crate::error`

## Threading / real-time notes

Synchronous, off the audio thread. Methods run on the MCP session thread under the server's session mutex; daemon probes (`InferClient::is_alive` and
status/turnaround) are short socket round trips. No real-time audio path is touched; no allocation or I/O is performed in callback context.

## Error handling

- Unknown voice ids and profiles return `ApiError::NotFound` with a pointer to `tts_voices`.
- Empty text or design prompts and unknown transcript formats return `ApiError::Validation`. Speed multipliers are clamped to 0.25-4.0; non-numeric
  values fail JSON type parsing before reaching the tool.
- Sandbox violations return `ApiError::SandboxEscape` (path traversal, symlinked ancestors, outside roots).
- Daemon failures degrade to the in-process built-in engines with a `tracing::warn!`; the result names the engine that actually ran.

## Testing

Inline `mod tests` in `crates/alp-api/src/voice.rs` cover: deterministic seeded voice listing, unknown-profile errors, WAV generation into `media/` with
honest engine naming (daemon-or-builtin agnostic), empty text and unknown voice rejection, Observe-tier denial, transcription round trip with the
placeholder-words admission, format and traversal rejection, and the designed-only voice design lifecycle (no cloned profiles, no consent records).

- `mod tests` in `crates/alp-api/src/voice.rs` (alpha.6) cover `voice_profile_update` (mutates stored name/tags/default params; unspecified parameter
  fields merge over stored values; the stored profile re-read carries the new values), the `voice_profile_lock`/`voice_profile_unlock` round trip
  (lock refuses updates until unlocked, unlock re-allows them), and `tts_batch` (one job per request, WAVs on disk, jobs queryable as `completed`,
  empty request list rejected).
- Inline `mod tests` in `crates/alp-api/src/audiobook.rs`: encode places one clip per segment on the `Audiobook` track and exports a real master
  (WAV file, `job_status` = `completed`), unknown voice rejected (`NotFound`), empty wrapped/bare scripts rejected (`Validation`).

## Known limits

- The built-in acoustic engine maps every voice id to a generic timbre; profile parameters (`VoiceParams`) do not steer synthesis until a backend consumes
  them. The `engine` field is the source of truth for what produced the audio.
- `voice_profile_lock`/`voice_profile_unlock` (alpha.6) flip the `locked` flag on the profile (see `crates/alp-voice/src/profile.rs`); enforcement of
  "locked = cannot change" against other agents is by convention here — the human GUI remains the primary enforcement surface for locks.
- `tts_generate` writes into `media/` but does not register a project source or insert a clip (no `media_import` yet).
- Transcription is non-streaming; the daemon's neural tier caps input at 300 s per call (longer input falls back to the built-in segmenter).
- `tts_script_generate`, `tts_regenerate`, `transcript_get`, and `export_transcript` are deferred (ADR 0026), as are `job_status`/`job_cancel` (nothing
  submits jobs yet).
- The voice store opens per call; hot loops over voice metadata should hold a `ProfileStore` directly.
