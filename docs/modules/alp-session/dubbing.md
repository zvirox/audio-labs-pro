# `alp-session` / `dubbing`

## Purpose

IPC bridge for the Dubbing Studio. Exposes four-stem separation, rate fitting and video remux to the desktop shell without making `alp-desktop` depend on inference or media internals.

## Public API

- `DUB_STEM_NAMES` (`vocals`, `drums`, `bass`, `other`).
- `StemSeparationRequestDto`, `StemFileDto`, `StemSeparationReport`, `separate_stems`.
- `RateFitRequestDto`, `RateFitReport`, `fit_rate`.
- `VideoRemuxRequestDto` (re-export of `alp_media::RemuxRequest`), `remux_video`.

## Data flow

1. Decode the requested source audio with `alp_media::decode`.
2. Stem separation connects to the local `alp-infer` daemon via `alp_voice::InferClient`, requires a loaded stem model, and writes four WAVs to the requested directory.
3. Rate fitting uses `alp_voice::fit_take` (WSOLA) and writes the fitted WAV.
4. Remux delegates to `alp_media::VideoRemuxer` with stream-copied video.

## Allowed dependencies

`alp-media` (decode/export/remux), `alp-voice` (inference client + rate fitting) and `serde` for DTOs. It does **not** depend on `alp-infer`; all model work crosses the socket protocol.

## Threading / real-time notes

These are synchronous IPC operations with file I/O and possibly long inference; call from Tauri command workers, never from the audio callback.

## Error handling

Missing files, a stopped daemon, an unloaded stem model, and export/remux failures return descriptive strings suitable for Tauri `Result<T, String>` handlers.

## Testing

Tests verify stem ordering and that a missing input file is reported rather than returning a fabricated report.

## Known limits

- Stem separation requires a compatible model and a running daemon; no model is bundled.
- `separate_stems` writes WAV files; loading them onto multitrack lanes is a separate UI/import step.
- Remux requires system FFmpeg and always stream-copies the video.
