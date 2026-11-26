# `alp-session` / `effects`

## Purpose

Frequency-response telemetry for the Effects workspace. Converts UI EQ band settings into the actual `alp-dsp` biquad response so the workspace draws the filter the mix engine uses, not a decorative curve.

## Public API

- `BiquadBandDto`: `filter_type`, `frequency_hz`, `q`, `gain_db`, `enabled`.
- `BiquadResponseDto`: `frequencies_hz`, `magnitude_db`, `phase_deg`.
- `compute_biquad_response(bands, sample_rate, points)`: pure log-frequency sweep over all enabled bands.

## Data flow

1. The Effects workspace sends its active stage band settings through `effects_get_response`.
2. The Tauri command calls `compute_biquad_response`, which designs each `alp_dsp::Coefficients` instance.
3. Magnitudes are multiplied (dB added) and phases summed across enabled bands.
4. The sampled arrays are returned to the canvas renderer; no project state is mutated.

## Allowed dependencies

`alp-dsp` for biquad design and response evaluation, and `serde` for IPC DTOs. No UI, audio-device, or inference dependencies.

## Threading / real-time notes

Pure computation on the IPC thread. It does not lock the session, read project state, or run on the audio callback.

## Error handling

Unknown filter-type strings fall back to `Peaking`. Point count is clamped to 16–512; zero sample rate yields a unity response through `Coefficients::response`. Non-finite band values are clamped inside biquad design.

## Testing

Unit tests cover disabled bands producing exact unity response and an enabled low-pass rolling off above its cutoff, with frequency/magnitude/phase arrays of equal length.

## Known limits

- Dynamics stages (compressor, gate, limiter) are not included in the static magnitude curve; their behaviour is level-dependent.
- Phase is the algebraic sum of band phases, wrapped to ±360 degrees, not an unwrapped group-delay plot.
- The analysis grid stops just below Nyquist to avoid degenerate response values.
