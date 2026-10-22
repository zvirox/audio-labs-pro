# 0009 — Peak pyramid cache format

- Status: accepted
- Date: 2026-10-22
- Spike: `spikes/audio-spike` (`peaks` mode)

## Context

The waveform must scroll a 1-hour source at 60 fps, and reopening a project must
be near-instant. Peaks are derived data and can be rebuilt.

## Decision

- Per source: 3 levels, base bucket 256 samples, each level ×4.
- Values stored as **i16 min/max pairs** (−32768..32767), not f32, halving size.
- Header: `ALPPEAKS` magic, u32 version, u32 sample rate, u64 frames, u32 level
  count; then per level `{u32 bucket_samples, u64 count, i16 pairs}`.
- Stored in the project's `peaks/` directory. The filename is derived from the
  source file stem plus an FNV-1a hash of (file size, mtime, sample rate); the
  header repeats the version, rate and frame count so a stale entry is rejected.
- Corrupt or stale caches are deleted and rebuilt; peaks are never required for
  correctness.

## Evidence

| Measurement | Result |
|---|---|
| 1-hour WAV (172.8 M frames) decode + peaks | 998 ms |
| cache size | 3.54 MB (3.38 MiB) |
| cache write | 3.1 ms |
| cache reload | 0.92 ms |
| bucket count | 885,938 across 3 levels |

## Consequences

- 30-minute, 10-track projects reload peaks in single-digit milliseconds.
- The UI only ever sees a public `PeakPyramid` type from `alp-media`.
- Cache invalidation is hash-driven; no migration needed for format changes
  (version bump + rebuild).
