# `alp-mcp` / `transport_tools`

## Purpose

Tool definitions for live transport control (`transport_state`, `transport_play`, `transport_pause`, `transport_stop`, `transport_seek`, `transport_set_volume`, `transport_set_loop`), kept separate so `tool_defs.rs` stays under the 1500-line guard.

## Public API

- `TRANSPORT_TOOLS`: crate-private list of all transport tool names.
- `definitions(tier) -> Vec<McpTool>`: build the schemas available at that permission tier.

## Data flow

1. `tools_for_tier` appends these definitions after the base, Edit and Export tools.
2. `transport_state` is available at Observe; the six mutating tools require Edit.
3. `accepts_expected_revision` excludes transport tools: they control live device state, not the project snapshot.

## Allowed dependencies

`alp_api::PermissionTier`, `serde_json`, and `crate::protocol::McpTool`. No new crate dependency.

## Threading / real-time notes

Definitions are pure data built once per `tools/list` request. No audio-thread work.

## Error handling

Schemas state the attach requirement in tool descriptions; actual refusal is produced by `transport_dispatch` in headless mode and by the live interceptor in attached mode.

## Testing

Covered by the 87-tool count assertion in `main.rs` and the headless transport checks in `scripts/mcp_eval.py` Suite 11.

## Known limits

- These tools do nothing in a headless session; they require `alp-mcp --attach` and a running desktop app with AI control enabled.
- Transport actions are not project edits and are not undoable through history tools.
