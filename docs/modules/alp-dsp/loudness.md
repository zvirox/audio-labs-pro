# `alp-dsp` / `loudness`

## Purpose

Implements ITU-R BS.1770-4 / EBU R128 loudness metering and target normalization gain calculations for podcasts (-16 LUFS), streaming (-14 LUFS), and broadcast standards.

## Public API

- `KWeighting`: `new`, `prepare`, `process`, `reset`
- `measure_lufs(buffer: &[f32], sample_rate: u32, channels: u16) -> f32`
- `calculate_normalization_gain(current_lufs: f32, target_lufs: f32, peak_linear: f32, ceiling_db: f32) -> f32`

## Data flow

1. Applies two-stage K-weighting filter (high shelf acoustic head model + high pass RLB filter).
2. Computes mean-square energy over the channel frames.
3. Converts weighted energy to integrated LUFS using the BS.1770 formula (`-0.691 + 10 * log10(sum_sq)`).
4. `calculate_normalization_gain` computes linear gain adjustment while clamping against a peak ceiling.

## Allowed dependencies

- `crate::biquad::{Biquad, FilterType}`
- Standard library math

## Threading / real-time notes

- `KWeighting::process` is real-time safe and operates in-place.
- `measure_lufs` allocates a temporary clone for filtering and is designed for offline/render and meter-polling contexts.

## Error handling

Returns `-70.0` LUFS for empty buffers, silent buffers ($< 10^{-12}$), or invalid rates/channel counts.

## Testing

Unit tests verify that full-scale 1 kHz sine waves measure within $\pm 1$ LUFS of ITU-R expectations (-3.01 LUFS) and that normalization gain never breaches specified peak ceilings.

## Known limits

- Integrated loudness calculation currently performs un-gated measurement; gated measurement (relative -10 dB gate) can be added as a refinement for long-form speech.
