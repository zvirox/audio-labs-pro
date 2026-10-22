# `alp-media::resample`

## Purpose

Offline sample-rate conversion with rubato’s FFT resampler, preserving the
interleaved channel layout and returning an owned `PcmBuffer`.

## Public API

`resample(buffer: &PcmBuffer, target_rate: u32) -> Result<PcmBuffer, MediaError>`

## Data flow

The function deinterleaves into per-channel vectors, asks rubato for a
`Fft` resampler sized to the channel count, processes the whole buffer, then
interleaves the result. A target rate equal to the source rate returns a clone.

## Allowed dependencies

`rubato`, `PcmBuffer`, `MediaError`. No ALP crate besides `alp-core` via errors.

## Threading / real-time notes

Allocates and performs an FFT over the whole buffer. Worker-thread only. The
engine calls it while preparing a scene, never in the callback.

## Error handling

Fft construction or processing failure becomes `MediaError::Resample`. An empty
input returns an empty buffer at the new rate.

## Testing

Tests cover identity output and resampling 44.1 kHz to 48 kHz with duration
preserved within two frames. Round-trip resampling and anti-alias response are
not yet tested.

## Known limits

- FFT resampling introduces a small amount of ringing; no minimum-phase or
  sinc option is exposed.
- Whole-buffer only; no streaming resampler.
- No dither or bit-depth conversion.
