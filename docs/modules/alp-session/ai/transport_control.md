# `alp-session` / `ai::transport_control`

## Purpose

Intercepts `transport_*` MCP tool calls on the UI thread in attached mode and drives the live `TransportController` instead of the scratch project snapshot.

## Public API

- `handle_transport_call(manager, request, response, session)` — crate-private entry point called from `ai::handle_mcp_request`.
- Tools handled: `transport_state`, `transport_play`, `transport_pause`, `transport_stop`, `transport_seek`, `transport_set_volume`, `transport_set_loop`.

## Data flow

1. `ai::handle_mcp_request` parses the JSON-RPC request and detects a `transport_` tool name.
2. The manager checks host generation, transport attachment, and (for mutations) the Edit permission tier.
3. The controller is locked briefly; the requested operation runs against the live `EditSession` where needed.
4. A `ToolCallResult` carrying the `TransportState` snapshot is sent back as `ControlResponse::Mcp`. No worker thread or project revision commit is involved.

## Allowed dependencies

`alp_api` (permission tier, control response), `alp_mcp` (JSON-RPC and tool result types, `live::tool_name`), `serde_json`, and the parent `ai` module. No new crate dependency.

## Threading / real-time notes

Runs on the UI tick thread, never on the real-time audio callback. The transport mutex is held only for the duration of one command; live playback state is read through `TransportController::state()`.

## Error handling

Missing transport, insufficient tier, poisoned lock, invalid arguments and transport errors return `isError: true` tool results with explanatory text rather than fabricating state. Unknown transport names are refused.

## Testing

Covered by the attached MCP socket round-trip test in `ai.rs` and the headless error checks in `scripts/mcp_eval.py` Suite 11.

## Known limits

- Headless `alp-mcp` has no transport; the shared tool catalog deliberately returns an "attach required" error there.
- Transport calls do not participate in project undo/redo; they are live device state, not project edits.
- If no desktop transport has been injected via `set_transport`, attached calls report "not attached" instead of falling through to the scratch worker.
