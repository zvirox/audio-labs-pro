# `alp-dsp` / `repair`

## Purpose

Targeted spectral repair on a 2D time–frequency selection. `SpectralNotchFilter` attenuates a narrow band around a centre frequency; `HarmonicSuppressor` attenuates a fundamental and its overtones. Both share a preallocated 1024-point STFT overlap-add engine so the audio path performs no per-block allocation up to the reserved buffer capacity.

## Public API

- `RepairMode`: `Notch` or `Harmonic` mask shape.
- `SpectralRepairConfig`: `mode`, `center_hz`, `bandwidth_q`, `fundamental_hz`, `harmonics`, `reduction_db`, `wet`.
- `SpectralRepair`: shared STFT engine with `new(config)`, `notch(..)`, `harmonic(..)`, `config()`, `is_enabled()`, and live setters for centre, Q, reduction, fundamental, harmonics, wet, and enabled state. Implements `Effect`.
- `SpectralNotchFilter`: `new(center_hz, bandwidth_q, reduction_db)` plus live `set_center_hz`, `set_bandwidth_q`, `set_reduction_db`, `set_enabled`.
- `HarmonicSuppressor`: `new(fundamental_hz, harmonics, bandwidth_q, reduction_db)` plus live `set_fundamental_hz`, `set_harmonics`, `set_reduction_db`, `set_enabled`.

## Data flow

1. `prepare(sample_rate, channels)` allocates per-channel input/OLA/output FIFOs, deinterleave scratch, and shared FFT scratch.
2. `process(buffer)` saves the dry signal, deinterleaves channels, and runs STFT → bin mask → inverse STFT with 50% overlap-add per channel.
3. The mask is a flat-top band with a 20% raised-cosine edge taper; attenuation is `10^(-reduction_db/20)` inside the band.
4. Wet/dry mix is applied back into the original interleaved buffer. Disabling the processor bypasses the input exactly.

## Allowed dependencies

`alp-core` (via `crate::Effect`) and the in-crate `denoise` constants for the shared FFT size. External crates: `realfft` (FFT planning and execution). No I/O, UI, or inference dependencies.

## Threading / real-time notes

Single-threaded `Effect` implementation. All FFT plans, windows, FIFOs, OLA buffers, and scratch vectors are created in `new`/`prepare`; `process` only resizes scratch to the incoming block length after capacity is reserved. No locks, logging, or filesystem access in the audio path.

## Error handling

Constructors clamp non-finite or out-of-range parameters (Q `>= 0.5`, reduction `>= 0`, wet `[0,1]`, harmonics `<= 16`). FFT planning errors are not returned by `Effect::process`; a failed transform simply passes that frame's bins through unchanged. `reset` clears all buffers without returning errors.

## Testing

Unit tests in `crates/alp-dsp/src/repair.rs` verify: a 1 kHz notch attenuates the selected band while preserving 3 kHz; a 220 Hz harmonic suppressor attenuates the fundamental and second overtone; disabled processing is bit-exact passthrough; and moving the centre frequency live changes the masked band. The test harness primes the STFT pipeline to account for its one-window latency.

## Known limits

- The FFT size is shared with `SpectralDenoiser` (1024), so bin resolution is ~46.9 Hz at 48 kHz; Q values below ~4 are needed to cover a tone at low frequencies.
- The mask is applied to whole bins; sub-bin frequency precision is not available.
- The 2D selection box itself is owned by `alp-session::SpectralSelectionDto`; this module accepts plain frequency/Q parameters and does not depend on session types.
- Scratch capacity is reserved for 4096 frames per channel; unusually large blocks may reallocate once.
