# `alp-mcp` / `studio_dispatch`

## Purpose

Dispatch helpers for the Phase 19 studio tools: EQ response, ACX QA, M4B export, marker removal, loop range and clip trim. Kept in a separate module so `dispatch.rs` stays under the 1500-line guard.

## Public API

All functions are crate-private and called from `dispatch.rs`:

- `dispatch_effects_response(arguments)`
- `dispatch_acx_qa_analyze(session, arguments)`
- `dispatch_audiobook_export_m4b(session, arguments)`
- `dispatch_marker_remove(session, arguments)`
- `dispatch_loop_range_set(session, arguments)`
- `dispatch_clip_trim(session, arguments)`

## Data flow

1. Require and parse every field by name; malformed values return a failure naming the field.
2. Deserialize nested DTOs with `serde_json` and call the matching `alp_api` operation.
3. Return `ToolCallResult::success` with a short human message plus the structured report, or `ToolCallResult::failure` with the API error.

## Allowed dependencies

`alp-api` DTOs and session, `serde_json`, and `crate::protocol`. No new crate dependency.

## Threading / real-time notes

Runs on the MCP server thread behind the session mutex. No audio-callback work is performed; M4B export and ACX decoding are bounded by the sandbox and workspace size.

## Error handling

Missing or mistyped arguments, empty chapter lists, sandbox escapes and API errors all produce plain-language failures. Required fields never fall back to zero values.

## Testing

Covered by the `alp-mcp` tool-surface tests and the Phase 19 end-to-end scenarios in `scripts/mcp_eval_scenarios.py`.

## Known limits

- `clip_trim` consumes frame indices, not seconds; callers must know the clip's source rate.
- `loop_range_set` requires both times or neither; partial input is rejected.
- M4B metadata is optional and defaults to empty strings.
