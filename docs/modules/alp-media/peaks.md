# `alp-media::peaks`

## Purpose

Builds and queries the three-level min/max peak pyramid used by the waveform
view. Level 0 stores 256-frame buckets; each higher level combines four buckets
of the level below, giving 1,024 and 4,096 frames per bucket.

## Public API

- Constants `BASE_BUCKET` (256), `LEVEL_RATIO` (4), `LEVEL_COUNT` (3),
  `CACHE_MAGIC` (`ALPPEAKS`), `CACHE_VERSION` (1)
- `PeakLevel::bucket_samples`, `pairs`, `len`, `is_empty`
- `PeakPyramid::from_parts`, `build_from_buffer`, `build_from_path`,
  `sample_rate`, `frames`, `levels`, `level_for(samples_per_pixel)`,
  `peak_between(level, start, end)`
- `PeakBuilder::push_chunk(chunk, channels)`, `finish(sample_rate)`, `frames`

## Data flow

`PeakBuilder` accepts decoded chunks (any size), averages channels to mono for
the peak envelope, accumulates min/max in i16 and emits buckets. `finish`
builds the two higher levels and validates geometry. `level_for` picks the
coarsest level whose buckets fit within the requested samples per pixel.
`peak_between` merges the buckets touching a frame range into one min/max pair.

## Allowed dependencies

`alp-media` PCM/decode, `alp-core` errors. No UI crate.

## Threading / real-time notes

Building allocates and scans audio; worker only. The finished pyramid is
immutable and cheap to query on the UI thread. Cache encoding is separate
(`cache.rs`).

## Error handling

Empty input, a zero channel count or inconsistent chunk dimensions return
`MediaError::InvalidBuffer`. `from_parts` validates matching levels and frame
counts.

## Testing

Tests cover three-level geometry on a tiny buffer, min/max sign, and
`level_for` selection at two zoom levels. Brute-force peak comparison and
channel mixing are not yet tested.

## Known limits

- Min/max only; no RMS or spectral data.
- i16 quantisation is sufficient for drawing but not for metering.
- Three fixed levels; very long timelines reuse level 2.
