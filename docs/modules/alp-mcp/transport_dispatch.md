# `alp-mcp` / `transport_dispatch`

## Purpose

Headless fallback for live transport tools. A headless MCP server has no audio transport, control socket or desktop token, so it refuses these calls explicitly instead of fabricating state.

## Public API

- `dispatch_transport(tool_name) -> ToolCallResult` (crate-private).

## Data flow

1. `dispatch.rs` routes any `transport_*` tool name to this helper.
2. The helper returns `ToolCallResult::failure` with attach guidance and the tool name.
3. In attached mode the desktop host intercepts these names before dispatch, so this path is never reached.

## Allowed dependencies

`crate::protocol::ToolCallResult` only.

## Threading / real-time notes

Pure function; no threading or audio work.

## Error handling

The error text names both the missing capability and the remedy (`alp-mcp --attach`, enable AI control). No neutral success is ever returned.

## Testing

Verified by `scripts/mcp_eval.py` Suite 11, which asserts `isError: true` for headless `transport_state` and `transport_play` calls.

## Known limits

- This module cannot control playback by design; the live implementation is in `alp_session::ai::transport_control`.
