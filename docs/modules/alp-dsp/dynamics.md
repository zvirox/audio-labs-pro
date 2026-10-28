# `alp-dsp` / `dynamics`

## Purpose

Provides dynamic range compression and noise gating tailored for spoken word, voice-over, and dialogue.

## Public API

- `Compressor`: `new`, `voice_default`, and `Effect` implementation.
- `NoiseGate`: `new`, `voice_default`, and `Effect` implementation.

## Data flow

1. `Compressor` measures frame peak amplitude in dB FS, applies static soft-knee compression characteristic,
   smooths gain reduction with attack/release ballistic filters, and adds makeup gain.
2. `NoiseGate` monitors signal energy against a threshold, holding open during brief speech pauses and
   smoothly attenuating room noise down to a target noise floor.

## Allowed dependencies

`alp-core::CoreError` and `crate::Effect`.

## Threading / real-time notes

Envelope updates and smoothing equations run per sample without memory allocations or locking.

## Error handling

Constructors validate finite parameters, positive ratios, and non-negative attack/release constants.

## Testing

Unit tests in `dynamics.rs` test gain reduction on loud signals and floor attenuation on sub-threshold noise.

## Known limits

- Single-band compression; multiband dynamic control is planned for post-v1.
