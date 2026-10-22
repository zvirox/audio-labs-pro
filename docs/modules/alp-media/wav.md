# `alp-media::wav`

## Purpose

Writes WAV files from a `PcmBuffer` in 16-bit, 24-bit PCM or 32-bit float.
Writes are atomic and sample-accurate.

## Public API

- `WavBitDepth::{Int16, Int24, Float32}`
- `WavBitDepth::bits`, `label`, `FromStr` (accepts `16`/`i16`/`int16`,
  `24`/`i24`/`int24`, `32`/`f32`/`float`/`float32`)
- `write_wav(path, buffer, bit_depth)`

## Data flow

A sibling temp file is created, hound writes the header and samples, then the
temp is renamed over the destination. Integer depths clamp to `[-1, 1]` and
scale by the maximum positive value; float writes values unchanged.

## Allowed dependencies

`hound`, `alp_core` atomic-file helpers, `PcmBuffer`, `MediaError`.

## Threading / real-time notes

Blocking disk I/O and per-sample conversion. Use a worker or the CLI. Not for
the audio callback.

## Error handling

Unsupported bit-depth strings return `MediaError::Encode`; hound failures
return `MediaError::Encode`; filesystem failures return `MediaError::Io`. A
failed write removes the temp file and leaves the old destination intact.

## Testing

Tests write 16-bit and float32 WAVs and check rate, channels and bit depth
through hound, plus parsing of bit-depth strings. 24-bit output and sample-value
round-trips are not yet unit-tested.

## Known limits

- WAV only; no FLAC/MP3/OGG export.
- No dither or noise shaping.
- Float WAV is 32-bit only.
