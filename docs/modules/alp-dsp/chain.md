# `alp-dsp` / `chain`

## Purpose

Provides sequential composite effect execution via `EffectChain` and pre-configured vocal mastering
through `VoiceCleanup`.

## Public API

- `EffectChain`: `new`, `add`, `len`, `is_empty`, and `Effect` implementation.
- `VoiceCleanup`: `build(mains_frequency: f32) -> EffectChain`.

## Data flow

`EffectChain::process` runs each contained effect sequentially over the interleaved buffer in place,
passing the output of stage $N$ to stage $N+1$.

## Allowed dependencies

Internal submodules of `alp-dsp` (`biquad`, `dynamics`, `eq`, `limiter`, `voice`), and `crate::Effect`.

## Threading / real-time notes

The chain delegates directly to each effect's real-time safe `process` method without per-block heap allocation.

## Error handling

Pre-validates component parameters upon construction.

## Testing

Unit tests verify safe processing of audio buffers without numerical overflow or NaN generation.

## Known limits

- Fixed serial topology; parallel routing or split-band splits are managed by custom callers.
