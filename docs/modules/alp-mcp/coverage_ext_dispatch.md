# `alp-mcp` / `coverage_ext_dispatch`

## Purpose

Dispatch helpers for the extended dubbing and repair MCP tools: rate fitting, video remux, spectral repair and time stretch. Separate from `dispatch.rs` to respect the 1500-line guard.

## Public API

All functions are crate-private:

- `dispatch_dubbing_fit_rate(session, arguments)`
- `dispatch_dubbing_remux(session, arguments)`
- `dispatch_dsp_spectral_repair(session, arguments)`
- `dispatch_dsp_time_stretch(session, arguments)`

## Data flow

1. Require each field by name; missing required values produce a failure naming the field.
2. Fill optional repair parameters with the DSP defaults (1000 Hz / 220 Hz / 4 harmonics / Q 12 / 24 dB).
3. Build the `alp_api` DTO and call the matching session operation.
4. Return a short message plus the structured report, or a plain-language failure.

## Allowed dependencies

`alp-api` DTOs and session, `serde_json`, and `crate::protocol`. No new crate dependency.

## Threading / real-time notes

Runs on the MCP server thread behind the session mutex. All work is offline file processing; no real-time audio callback is involved.

## Error handling

Missing arguments, invalid ratios/modes, sandbox escapes and media errors are reported as `ToolCallResult::failure`. No zero-value defaults are used for required fields.

## Testing

Covered by the `alp-mcp` tool-surface tests and Suite 10 of `scripts/mcp_eval.py`.

## Known limits

- Stem separation is not exposed: it requires `alp-infer` and a loaded model.
- Video remux requires external `ffmpeg`/`ffprobe` at runtime.
