# `alp-cli` / `commands/dsp`

## Purpose

Exposes DSP capabilities headlessly: Voice Cleanup pipeline execution and ITU-R BS.1770
integrated loudness (LUFS) analysis with normalization gain calculation.

## Public API

- `measure(input: &Path, json: bool) -> Result<()>`
- `cleanup(input: &Path, output: &Path, mains_freq: f32) -> Result<()>`
- `process_effect(input: &Path, output: &Path, effect_name: &str, effect: Box<dyn Effect>) -> Result<()>`

## Data flow

1. `measure` decodes input media with `alp-media`, passes samples to `alp_dsp::measure_lufs`,
   computes true peak and normalizations for podcast/streaming/broadcast, and prints text or JSON.
2. `cleanup` decodes input, prepares the `VoiceCleanup` chain for the stream sample rate/channel layout,
   processes interleaved samples in place, and exports a 24-bit WAV master.

## Allowed dependencies

`alp-core`, `alp-media`, `alp-dsp`, `anyhow`, and `std::path::Path`.

## Threading / real-time notes

Runs on the CLI command thread. File I/O and offline DSP processing are fully synchronous.

## Error handling

Propagates decoding, file creation and export errors with full contextual error chains (`anyhow`).

## Testing

Covered in acceptance harness `scripts/phase5-demo.sh` with verify-checks on limiter ceiling and bit-exact reproducibility.

## Known limits

- Offline file processing only; streaming input from pipe is not supported.
