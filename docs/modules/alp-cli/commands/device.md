# `alp-cli::commands::device`

## Purpose

Implements `devices` and `record`: enumerate output/input devices and capture
the default input to WAV.

## Public API

- `devices(json: bool)`
- `record(output: &Path, seconds: f64)`

## Data flow

`devices` calls `alp_engine::list_devices`, prints a direction-tagged table or a
JSON array, and marks defaults. `record` calls `alp_engine::record_to_wav` and
prints frames, rate, channels, callbacks, dropped samples and elapsed time.

## Allowed dependencies

`alp-engine`, `anyhow`, `serde_json`, `std::path`.

## Threading / real-time notes

`devices` probes hardware and may block briefly. `record` blocks for the requested
duration while the engine's writer thread handles disk I/O.

## Error handling

No devices is not an error for `devices` (it prints a notice). A missing input
device or stream failure is returned with context and exits non-zero. A
`stream_error` report is converted into a failure after printing diagnostics.

## Testing

No device test runs in CI. `scripts/phase1-demo.sh --play` and `--record` exercise
the hardware paths on a real machine.

## Known limits

- Default device only.
- No channel/rate selection, monitoring or level display.
- `record` has no countdown or cancellation.
