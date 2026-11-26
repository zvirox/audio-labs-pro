# `alp-dsp` / `true_peak`

## Purpose

Inter-sample true-peak estimation for ACX and broadcast mastering compliance. Sample peak alone can miss peaks between samples; this module probes quarter-sample offsets with Catmull-Rom interpolation.

## Public API

- `TRUE_PEAK_OVERSAMPLING = 4`.
- `measure_true_peak_dbfs(samples, sample_rate, channels) -> f32`: estimated true peak in dBFS, clamped to `-120..+6`.

## Data flow

1. Validate buffer, rate, and channel count.
2. For each frame of the first two channels, evaluate the current sample plus three quarter-sample offsets using neighbouring samples.
3. Track the maximum absolute amplitude and convert it to dBFS.

## Allowed dependencies

No internal crate dependencies. Pure Rust floating-point math.

## Threading / real-time notes

Pure synchronous function; suitable for offline analysis. It allocates nothing and can be called from an API or worker thread. It is not intended for the audio callback.

## Error handling

Empty buffers, zero sample rate, or zero channels return `-120.0 dBFS`. Silent buffers return the same floor. Non-finite samples are propagated into the interpolation and may yield a floored/upper-clamped result.

## Testing

Tests verify the silence floor, a full-scale sine near `0 dBFS`, and a half-scale sine near `-6 dBFS`.

## Known limits

- Catmull-Rom quarter-sample probing is an approximation of the 4× oversampled true peak required by ITU-R BS.1770-4; it does not apply the full reconstructed-filter response.
- Only the first two channels are analysed (stereo masters).
- The result is clamped at `+6 dBFS` for display stability.
