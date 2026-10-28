# `alp-dsp` / `biquad`

## Purpose

Provides Direct Form I/II biquad digital IIR filters based on Robert Bristow-Johnson's
Audio EQ Cookbook for standard equalizer and filter shapes.

## Public API

- `FilterType`: `HighPass`, `LowPass`, `Peaking`, `LowShelf`, `HighShelf`, `Notch`, `BandPass`
- `Coefficients`: `identity`, `design`
- `ChannelState`: per-channel filter delay registers
- `Biquad`: `new`, `prepare`, `set_parameters`, `reset`, `process`

## Data flow

1. `Coefficients::design` converts frequency, Q, sample rate, and gain into normalized biquad poles/zeros.
2. `Biquad::process` processes interleaved multi-channel audio through independent channel state memory,
   flushing denormals to zero.

## Allowed dependencies

Rust standard library only (`std::f32::consts::PI`).

## Threading / real-time notes

`process` is zero-allocation, lock-free, and real-time safe for execution inside the audio callback.
Channel memory is pre-allocated during `prepare`.

## Error handling

Frequencies are clamped below Nyquist; zero sample rate yields an identity filter.

## Testing

Unit tests in `biquad.rs` verify low-frequency attenuation in high-pass filters and passband fidelity in low-pass filters.

## Known limits

- Fixed second-order slope (12 dB/octave); higher orders are constructed by cascading stages.
