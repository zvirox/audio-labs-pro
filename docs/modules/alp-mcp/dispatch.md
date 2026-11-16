# `alp-mcp::dispatch`

## Purpose

Maps a tool name plus raw JSON arguments onto the real `alp-api` operation that
produces the tool's named artefact. This is the layer where the "do what it
claims" rule is enforced arm by arm: every arm either returns the structured
report the tool documents or a failure explaining what went wrong and which
call to make next. Nothing here invents a success.

## Public API

- `dispatch(session: &mut ApiSession, tool_name: &str, arguments: &Value) -> ToolCallResult` —
  the single entry point; one arm per advertised tool.
- `page_request(arguments) -> Result<PageRequest, String>` — parses the shared
  `limit`/`cursor` arguments for the list tools.

## Data flow

1. `server.rs` takes the session lock, admits the call through the rate
   limiter, applies the `expected_revision` guard, then calls `dispatch`.
2. `dispatch` parses arguments per arm and calls exactly one `ApiSession`
   method (or, for `edit_batch`/`export_*`, a small sequence of them).
3. Every arm wraps its report in `ToolCallResult::success` with a one-line
   human summary, or `ToolCallResult::failure` with the cause and recovery.

## Allowed dependencies

- `alp_api::{ApiSession, ExportPreset}`
- `serde_json::{Value, json}`
- `crate::protocol::ToolCallResult`

No I/O of its own: all file, job and analysis work happens inside `alp-api`.

## Threading / real-time notes

Called with the session mutex already held, on the server thread. Never touches
the audio callback path; long operations (TTS, STT, render) run through the
session's job manager and report a `job_id`.

## Error handling

Missing required arguments `return` a failure that names the argument instead
of defaulting to a zero value that would edit the wrong thing. Unknown ids
(export presets, effect/preset ids, tool names) fail with the valid
alternatives listed. Structured failures keep `isError: true` and set no
`structuredContent`; partial successes (for example a batch where the report is
real but a note applies) keep their structured payload and add the note as
text.

## Testing

Exercised end-to-end through the 30 tests in `crates/alp-mcp/src/main.rs`,
which call `dispatch` via `tools/call` on real sessions, including:

- `every_advertised_tool_is_dispatched` — the dispatch table covers the whole
  advertised surface;
- `edit_batch_applies_real_operations_and_collapses_into_one_undo` and
  `a_failing_edit_batch_reports_no_changes` — atomicity of the batch arm;
- `render_waveform_returns_the_art_it_computed` and
  `render_spectrogram_returns_a_real_time_frequency_map` — the analysis arms
  return the artefact they name;
- `trim_silence_through_mcp_actually_shortens_the_timeline` — smart edits
  mutate the project for real.

## Known limits

- Arms are matched on tool name strings; the parity test
  (`every_advertised_tool_is_dispatched`) is what keeps the table and the
  schema list from drifting apart.
- Argument parsing is hand-rolled per arm (`Value::as_str`/`as_f64`) rather
  than a derive-based schema deserializer, so a wrong-typed argument surfaces
  as the tool's own validation failure rather than a schema error.
- File-writing arms are synchronous: only the TTS/STT/script tools go through
  the job manager.
