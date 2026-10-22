# `alp-media::synthetic`

## Purpose

Generates deterministic test audio without external assets. The fixture is used
by CI, the CLI `gen` command and the Phase 1 acceptance scripts.

## Public API

`write_test_tone(output, seconds, sample_rate, channels, frequency, amplitude)`

## Data flow

For each frame the generator computes a base sine plus a second partial, a slow
amplitude envelope and a deterministic LCG hiss (no global RNG). Channels are
written identically. It streams samples directly into a 16-bit PCM WAV temp file
and renames it into place, so a one-hour fixture does not require one hour of
RAM.

## Allowed dependencies

`wav`, `PcmBuffer`, `MediaError`. No ALP crate besides `alp-core` via errors.

## Threading / real-time notes

Generates and writes on the call thread; it does not allocate a full buffer.
It is not used by the audio callback.

## Error handling

Duration, channel count, frequency and amplitude are validated explicitly and
return `MediaError::InvalidBuffer`. Encoding failures return `MediaError::Encode`.
Existing files are replaced atomically.

## Testing

Tests generate a stereo tone, probe/decode it and check rate, channels,
frame count and peak, plus rejection of invalid duration, channels and
amplitude. Byte-for-byte determinism is not asserted.

## Known limits

- One shared waveform for every channel (no stereo width).
- The “noise” is a deterministic arithmetic sequence, not a real noise model.
- 16-bit output only.
