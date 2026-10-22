# `alp-media::error`

## Purpose

One error type for decoding, resampling, buffer construction, caching and WAV
encoding. It also wraps `CoreError` for atomic-write failures.

## Public API

`MediaError` variants: `Io`, `Decode`, `Unsupported`, `InvalidBuffer`,
`InvalidCache`, `Resample`, `Encode`, `Core`.

## Data flow

Low-level modules return `MediaError`; callers either display it or wrap it in
`EngineError`/`anyhow`. `#[from]` conversions make `?` ergonomic for I/O and
`CoreError`.

## Allowed dependencies

`std::io`, `thiserror`, `alp-core`.

## Threading / real-time notes

Plain error values. Formatting allocates; never do it in the audio callback.

## Error handling

Messages include the offending format or cache detail. Corrupt caches are
reported, then deliberately treated as a miss by `PeakCache::load_or_build`.

## Testing

Tests cover the validation variants reached by PCM dimensions, synthetic
arguments and corrupt cache. Truncated-WAV and unsupported-extension variants
are not yet asserted directly.

## Known limits

- Error strings are not localised.
- No error codes for programmatic recovery beyond the variant.
