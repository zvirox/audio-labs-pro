# `alp-mcp` / `server`

## Purpose

The primary MCP request dispatcher routing protocol methods, tool executions, and resource/prompt queries to `alp-api::ApiSession`.

## Public API

- `McpServer::new(session, tier) -> Self`
- `McpServer::handle_request(&self, req: JsonRpcRequest) -> JsonRpcResponse`

## Data flow

1. Accepts a `JsonRpcRequest`.
2. Inspects method:
   - `initialize` $\to$ returns server info and capabilities.
   - `server/discover` $\to$ returns spec revision metadata.
   - `tools/list` $\to$ filters tool registry by granted `PermissionTier`.
   - `tools/call` $\to$ executes target action via `ApiSession` and wraps output into `ToolCallResult`.
   - `resources/*` $\to$ returns URIs and contents.
   - `prompts/*` $\to$ returns workflow templates.
3. Produces a correlated `JsonRpcResponse`.

## Allowed dependencies

- `std::sync::{Arc, Mutex}`
- `serde_json::{Value, json}`
- `alp_api::{ApiSession, ExportPreset, PermissionTier}`
- `crate::protocol::{JsonRpcRequest, JsonRpcResponse, McpTool, ToolCallResult}`

## Threading / real-time notes

Acquires internal session mutex during tool calls; operations are serialized to guarantee consistent project revision sequencing.

## Error handling

Unknown methods produce JSON-RPC `-32601` method not found errors. Tool execution failures are safely trapped and returned as `isError: true` results.

## Testing

Covered by unit tests in `main.rs` and the Phase 10 demo suite.

## Known limits

- Single session per server instance.
