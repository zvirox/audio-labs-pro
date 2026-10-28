# `alp-mcp` / `protocol`

## Purpose

Defines JSON-RPC 2.0 and Model Context Protocol envelope types, tool schemas, and content responses.

## Public API

- `JsonRpcRequest`: incoming request envelope.
- `JsonRpcResponse`: outgoing response envelope.
- `JsonRpcError`: structured error description.
- `McpTool`: tool metadata and JSON Schema.
- `ToolCallResult`: structured and text content results.

## Data flow

1. Deserializes client requests from JSON strings.
2. Formats tool call results with text summaries and structured JSON representations.

## Allowed dependencies

- `serde::{Deserialize, Serialize}`
- `serde_json::Value`

## Threading / real-time notes

Pure serializable message envelopes with no external state.

## Error handling

Provides constructor helpers for protocol errors and tool-level failure payloads.

## Testing

Covered by serialization round-trips in `alp-mcp` tests.

## Known limits

- Conforms to standard JSON-RPC 2.0 conventions.
