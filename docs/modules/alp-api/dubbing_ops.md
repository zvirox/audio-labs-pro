# `alp-api` / `dubbing_ops`

## Purpose

Automation wrappers for the two dubbing operations that do not require the inference daemon: rate fitting and video remux.

## Public API

- `DubbingFitRequestDto`, `DubbingFitReport`, `ApiSession::dubbing_fit_rate(&request)`.
- `DubbingRemuxRequestDto`, `ApiSession::dubbing_remux(&request) -> Result<RemuxReport, ApiError>`.

## Data flow

1. Require the Export tier and validate the target duration.
2. Canonicalise all input/output paths through `WorkspaceSandbox::validate_path`.
3. Decode the take and delegate to `alp_voice::fit_take`; export the result as 16-bit WAV.
4. Remux video with `alp_media::VideoRemuxer` using sandbox-checked paths.

## Allowed dependencies

`alp-voice` (rate fitting), `alp-media` (decode/export/remux), `alp-core` (project paths), `serde`, and existing `alp-api` helpers. `alp-infer` is deliberately not a dependency.

## Threading / real-time notes

Synchronous filesystem/media work on automation worker threads. No audio-callback involvement.

## Error handling

Invalid durations, sandbox escapes, decode/export failures and remux failures return typed `ApiError` variants. Remux errors include missing external `ffmpeg`/`ffprobe` diagnostics.

## Testing

Tests verify a real fitted WAV is produced, zero target duration is rejected, and a missing video path produces an error without panicking.

## Known limits

- Four-stem separation is intentionally unavailable here: it needs `alp-infer` and a loaded `demucs-4stem` model.
- Video remux requires external `ffmpeg`/`ffprobe` at runtime.
