# `alp-dsp` / `eq`

## Purpose

Provides a multi-band parametric equalizer featuring high-pass filtering, low/high shelving, and
peaking bell filters.

## Public API

- `EqBand`: `new`, `filter_type`, `frequency`, `q`, `gain_db`, `enabled`
- `ParametricEq`: `voice_default`, `with_bands`, `set_band`, `set_band_enabled`, and `Effect` implementation

## Data flow

Processes incoming interleaved buffers through an array of cascade biquad filters, applying frequency
sculpting in order.

## Allowed dependencies

`crate::biquad`, `crate::Effect`.

## Threading / real-time notes

Filter coefficient updates and per-channel sample processing are zero-allocation and lock-free.

## Error handling

Filter parameter mutations clamp frequencies below Nyquist and bound Q factors.

## Testing

Unit tests verify multi-channel stability and high-shelf boost on sample buffers.

## Known limits

- Maximum of 5 bands in default voice configuration; additional bands can be supplied via `with_bands`.
