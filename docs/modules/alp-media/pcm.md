# `alp-media::pcm`

## Purpose

The canonical in-memory audio buffer: interleaved f32 samples with a sample
rate and channel count. Every decoder, resampler, mixer and WAV writer speaks
this type.

## Public API

- `PcmBuffer::new(sample_rate, channels, samples)`
- `silence(sample_rate, channels, frames)`
- `sample_rate`, `channels`, `frames`, `duration_secs`
- `samples`, `samples_mut`, `into_samples`, `sample(frame, channel)`
- `invert`, `peak`, `rms`

## Data flow

`new` validates a non-zero rate/channel count and that the sample count is a
multiple of the channel count. `silence` allocates zeroed samples. Accessors
never copy except `into_samples`, which consumes the buffer.

## Allowed dependencies

`MediaError` only. No ALP crate beyond `alp-core` through the error type.

## Threading / real-time notes

Construction allocates. `samples_mut` gives exclusive access, so the buffer is
safe to fill on one worker and then share immutably (`Arc`) with the engine.
No interior locks.

## Error handling

Invalid dimensions return `MediaError::InvalidBuffer`. `sample` returns `None`
when out of range. Statistics return `0.0` for empty buffers.

## Testing

Tests cover dimension validation, duration, silence construction and
peak/RMS. Sample indexing and inversion are implemented but not tested.

## Known limits

- f32 only; no planar or integer storage.
- No stride or offset view.
- `PcmBuffer` is not `Copy`; large buffers should be moved or shared via `Arc`.
