# `alp-core::time`

## Purpose

Audio-time helpers. All timeline positions are integer frames at the project
sample rate; seconds exist only at the UI and CLI boundary.

## Public API

- `FrameRange { start, end }` with `new`, `len`, `is_empty`, `contains`
- `frames_to_secs(frames, sample_rate)`
- `secs_to_frames(seconds, sample_rate)`

## Data flow

`FrameRange::new` rejects `end < start`. `len` is `end - start`. `contains` is
half-open (`start <= frame < end`). `secs_to_frames` clamps negative/non-finite
input to zero and rounds to the nearest frame; `frames_to_secs` divides by the
rate.

## Allowed dependencies

`std`, `serde`, `CoreError`. No other ALP crate.

## Threading / real-time notes

Pure arithmetic; safe anywhere. No allocation.

## Error handling

Only `FrameRange::new` is fallible (`CoreError::InvalidRange`). Conversions
saturate rather than panic.

## Testing

Tests cover inverted ranges, zero-length ranges and whole-second conversion
round-trips. Clamping of non-finite input is covered by code review, not a
dedicated test yet.

## Known limits

- `u64` frames cap a project at about 12 million years at 48 kHz, far beyond
  practical use, but `f64` precision degrades above 2^53 frames.
- No SMPTE timecode support.
