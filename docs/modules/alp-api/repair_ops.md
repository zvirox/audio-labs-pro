# `alp-api` / `repair_ops`

## Purpose

Offline spectral repair (notch/harmonic) and WSOLA time-stretch wrappers using the same DSP implementations as the real-time engine.

## Public API

- `SpectralRepairRequestDto`, `SpectralRepairReport`, `ApiSession::dsp_spectral_repair(&request)`.
- `TimeStretchRequestDto`, `TimeStretchReport`, `ApiSession::dsp_time_stretch(&request)`.

## Data flow

1. Require the Export tier; validate mode, frequencies, Q, reduction and stretch ratio.
2. Sandbox-check input and output paths.
3. Decode to `PcmBuffer`, build the `alp_dsp::Effect`, prepare it for the stream format and process interleaved chunks.
4. Export a 16-bit WAV and report frames/rate or resulting duration.

## Allowed dependencies

`alp-dsp` (effects and `Effect` trait), `alp-media` (decode/export/time-stretch), `alp-core`, `serde`, and existing `alp-api` helpers.

## Threading / real-time notes

All processing runs synchronously on an automation worker; chunks are capped at 4096 interleaved samples. No real-time callback is touched.

## Error handling

Unknown modes, invalid Q/frequency/ratio, sandbox escapes and media errors return typed `ApiError` values before any output file is written.

## Testing

Tests cover notch and harmonic repairs on generated tones, a real time stretch, and rejection of an unknown repair mode.

## Known limits

- Repair is an offline file-to-file pass, not a live insert on a track's effect chain.
- Output is always 16-bit WAV; other delivery formats require a follow-up export.
