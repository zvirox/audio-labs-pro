# `alp-media` / `compare`

## Purpose

Null-test helper that subtracts a rendered buffer from a reference and reports
the largest and RMS differences.

## Public API

- `NullReport { reference_frames, rendered_frames, compared_frames, max_abs_diff, rms_diff }`
- `NullReport::passed(tolerance)`
- `compare(reference, rendered) -> Result<NullReport, MediaError>`

## Data flow

Callers decode both files, resample the rendered file to the reference rate, and
pass same-layout buffers. `compare` walks the shorter interleaved slice and
reports a length mismatch as a failed null test.

## Allowed dependencies

`alp-media::PcmBuffer` and `MediaError` only.

## Threading / real-time notes

Offline comparison on the CLI/worker thread. It performs no I/O and no
allocation beyond the report.

## Error handling

Sample-rate or channel-count mismatches are rejected. Length mismatches do not
error; `passed` returns false so the caller can report both lengths.

## Testing

Tests cover identical buffers, sub-tolerance differences, length mismatch and
layout mismatch.

## Known limits

- The helper does not resample or remix; the caller must do that first.
- There is no perceptual or per-channel report yet.
