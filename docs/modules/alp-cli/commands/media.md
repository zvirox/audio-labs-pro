# `alp-cli::commands::media`

## Purpose

Implements `gen`, `decode` and `peaks`: create fixtures, inspect media and
build/reopen peak caches with timings.

## Public API

- `generate(output, seconds, sample_rate, channels, frequency, amplitude)`
- `decode_file(input, json)`
- `peaks(input, cache_dir, json)`

## Data flow

`generate` streams a deterministic 16-bit WAV through `write_test_tone` and
prints size/time. `decode_file` probes, decodes, computes peak/RMS and prints a
text or JSON report. `peaks` selects `--cache-dir` or `peaks/` beside the file,
calls `PeakCache::load_or_build`, and reports hit/miss, levels, frame count and
elapsed time.

## Allowed dependencies

`alp-media`, `anyhow`, `serde_json`, `std::time`, `std::path`.

## Threading / real-time notes

All synchronous. Decoding and peak building are CPU/IO heavy; one-hour fixtures
take about a second to build a cache in release mode.

## Error handling

Probe/decode/cache failures are wrapped with the file path. JSON is printed only
after the operation succeeds, so a partial object is never emitted. Invalid
numbers/arguments are rejected by clap or the media layer.

## Testing

`scripts/phase1-demo.sh` asserts a first-run miss and a second-run cache hit on a
one-hour WAV, and decodes the result. `cargo test --workspace` covers the media
layer underneath.

## Known limits

- `peaks` uses the file’s parent directory by default, not a project root.
- No waveform image or JSON dump of individual buckets.
- `decode` reports symphonia’s debug codec name.
