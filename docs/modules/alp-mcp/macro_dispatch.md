# `alp-mcp` / `macro_dispatch`

## Purpose

Dedicated dispatch helper for the `dsp_macro_execute` tool, keeping `dispatch.rs` under the 1500-line guard while preserving explicit argument validation and failure reporting.

## Public API

- `dispatch_dsp_macro(session: &mut ApiSession, arguments: &Value) -> ToolCallResult` (crate-private).

## Data flow

1. Require `input_dir` and `output_dir` and a non-empty `steps` array.
2. Convert each step's `params` object to a `BTreeMap<String, f64>` and build `MacroStep` values.
3. Construct `MacroRequest` and call `ApiSession::dsp_macro_execute`.
4. Return a structured `MacroJobReport` or a plain-language failure.

## Allowed dependencies

`alp-api` (session + macro DTOs) and `serde_json`. No new dependency edge.

## Threading / real-time notes

Runs on the MCP server thread; no audio-callback work. The underlying macro batch runs synchronously but is exposed as a cancellable job.

## Error handling

Missing directories, missing step ids, invalid parameter objects, and API errors all return `ToolCallResult::failure` with the offending argument named. No zero-value defaults are used for missing fields.

## Testing

Covered indirectly by the MCP tool-surface tests in `main.rs`, which assert every advertised tool is dispatched and annotated.

## Known limits

- This helper exists only to satisfy the file-length guard; it is not a public API.
