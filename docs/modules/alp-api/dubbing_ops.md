# `alp-api` / `dubbing_ops`

## Purpose

Automation wrappers for dubbing workflows: four-stem separation (via the isolated inference daemon), WSOLA rate fitting, and ffmpeg stream-copy video remux.

## Public API

- `ApiStemFileDto`, `DubbingStemsRequestDto`, `DubbingStemsReport`, `ApiSession::dubbing_separate_stems(&request) -> Result<DubbingStemsReport, ApiError>`.
- `DubbingFitRequestDto`, `DubbingFitReport`, `ApiSession::dubbing_fit_rate(&request)`.
- `DubbingRemuxRequestDto`, `ApiSession::dubbing_remux(&request) -> Result<RemuxReport, ApiError>`.

## Data flow

1. Require the Export tier and canonicalize input/output paths through `WorkspaceSandbox::validate_path`.
2. For `dubbing_separate_stems`, decode the input audio, probe the running `alp-infer` daemon via `alp_voice::InferClient` (auto-loading `demucs-4stem` when ready/available), run `separate_stems`, and export the 4 stems (`vocals`, `drums`, `bass`, `other`) into `output_dir`.
3. For `dubbing_fit_rate`, decode the take, delegate to `alp_voice::fit_take`, and export 16-bit WAV.
4. For `dubbing_remux`, remux video with `alp_media::VideoRemuxer` using sandbox-checked paths.

## Allowed dependencies

`alp-voice` (`InferClient` daemon IPC + rate fitting), `alp-media` (decode/export/remux), `alp-core` (project paths), `serde`, and existing `alp-api` helpers. `alp-infer` is deliberately not a dependency (`check-deps.sh`).

## Threading / real-time notes

Synchronous filesystem/media/IPC work on automation worker threads. No audio-callback involvement.

## Error handling

Sandbox escapes, invalid formats/durations, unavailable inference daemon or missing `demucs-4stem` weights, decode/export errors, and missing external `ffmpeg`/`ffprobe` return typed `ApiError` variants.

## Testing

Tests verify a real fitted WAV is produced, zero target duration is rejected, a missing video path returns an error cleanly, and `dubbing_separate_stems` enforces sandbox and daemon/model availability errors.

## Known limits

- `dubbing_separate_stems` requires a running `alp-infer` daemon with `demucs-4stem` weights installed (no model weights are bundled in the repo).
- Video remux requires external `ffmpeg`/`ffprobe` at runtime.
