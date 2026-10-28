# `alp-dsp` / `limiter`

## Purpose

Provides a peak limiter processor with instantaneous attack and smooth exponential release to prevent digital clipping while preserving dynamic range.

## Public API

- `Limiter`: `new`, `threshold`, `prepare`, `process`, `reset`, `name`
- Implements `crate::Effect`

## Data flow

1. Computes instantaneous peak across all channels per frame.
2. If peak exceeds threshold, immediately attenuates gain to `threshold / peak`.
3. If signal falls below threshold, smoothly recovers towards unity gain according to configured release time.
4. Multiplies frame samples by gain factor and clamps to threshold.

## Allowed dependencies

- `alp_core::CoreError`
- `crate::Effect`
- Standard library math

## Threading / real-time notes

`process` is zero-allocation and lock-free, fully safe for execution inside the real-time audio callback.

## Error handling

`new` returns `CoreError::Validation` if threshold, attack, or release values are NaN, infinite, or negative.

## Testing

Unit tests in `limiter.rs` verify that signal amplitudes exceeding threshold are clamped without clipping or audio dropouts, and that sub-threshold signals pass unaffected.

## Known limits

- Single-band limiter without multi-band split.
- Uses sample-peak detection rather than true oversampled inter-sample peak detection.
