# `alp-media::cache`

## Purpose

Persists and reloads the three-level peak pyramid. The cache is derived data:
if it is missing, stale or corrupt it is rebuilt from the source, never trusted
blindly.

## Public API

- `CacheStatus::{Hit, Miss}`
- `PeakCache::new(root)`, `root`, `entry_path(source, sample_rate)`,
  `load(cache_path)`, `load_or_build(source)`, `store(path, pyramid)`

`load_or_build` returns `(PeakPyramid, CacheStatus, PathBuf)`.

## Data flow

`entry_path` fingerprints the source by size, modified time and sample rate,
then derives a deterministic filename. `load` validates the `ALPPEAKS` magic,
version, sample rate, frame count and bucket geometry. `load_or_build` returns a
hit when valid; otherwise it decodes, builds and atomically stores a new cache.
`store` writes a sibling temp file and renames it.

## Allowed dependencies

`alp-core` (`write_atomic`) and other `alp-media` modules. `std` I/O.

## Threading / real-time notes

Blocking file I/O and decoding; worker-thread only. The pyramid is immutable
after load. Never touch the cache from the audio callback.

## Error handling

I/O failures are `MediaError::Io`; malformed data is `MediaError::InvalidCache`.
A failed cache write is returned as an error even though the pyramid was built;
the caller can retry or continue without caching.

## Testing

Tests cover a miss followed by a hit returning the same pyramid, and a
truncated file falling back to a rebuild. Path stability and sample-rate
changes are not dedicated tests yet.

## Known limits

- Fingerprinting uses size, mtime and rate; equal-size edits with a preserved
  mtime are not detected.
- Cache entries are not garbage-collected.
