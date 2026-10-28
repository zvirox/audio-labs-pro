# `alp-voice` / `reference`

## Purpose

Performs acoustics and quality validation on reference audio recordings used for voice cloning, verifying speech duration, peak headroom, clipping, noise floor, and silence ratio against strict quality standards.

## Public API

- `ReferenceQualityReport`: `MIN_DURATION_SECS`, `MAX_NOISE_FLOOR_DBFS`, `MAX_SILENCE_RATIO`, fields
- `analyze_reference_quality(buffer: &PcmBuffer) -> ReferenceQualityReport`

## Data flow

1. Computes peak dBFS, RMS energy, and counts full-scale clipping spikes across input PCM samples.
2. Evaluates noise floor across 20 ms blocks (10th percentile lowest energy) and computes silence ratio.
3. Generates human-actionable feedback if gates fail.

## Allowed dependencies

- `alp-media::PcmBuffer`
- `serde`, Standard library

## Threading / real-time notes

Operates in $O(N)$ linear time over the input buffer on background worker threads.

## Error handling

Returns a valid report across empty or malformed buffers with appropriate advisory messages.

## Testing

Unit tests in `reference.rs` test clean speech bursts against quality thresholds and verify that clipped or too-short samples fail with specific actionable advice.

## Known limits

- Threshold and percentile based; deep spectral noise estimation can be added when neural denoisers are active.
