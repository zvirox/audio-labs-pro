# `alp-mcp`

## Purpose

The local Model Context Protocol (MCP) server for Audio Labs Pro. Enables AI models, Claude Desktop, and IDE agents to inspect projects, perform non-destructive edits, analyze loudness, and export master audio.

## Public API

- Binary executable `alp-mcp`.
- Protocol support for MCP specification (2026-07-28).
- Dispatches standard JSON-RPC 2.0 messages over standard I/O (stdio).

## Data flow

1. Reads line-delimited JSON-RPC requests from standard input.
2. Deserializes request into `JsonRpcRequest`.
3. Dispatches methods (`initialize`, `server/discover`, `tools/list`, `tools/call`, `resources/list`, `resources/read`, `prompts/list`, `prompts/get`) via `McpServer`.
4. Writes JSON-RPC response objects to standard output.

## Allowed dependencies

- `alp-core`
- `alp-api`

## Threading / real-time notes

Runs as a separate standalone headless process communicating with `alp-api` via thread-safe Mutex guards.

## Error handling

Protocol parse errors return JSON-RPC standard error codes (-32700, -32601). Tool-level errors return `isError: true` with actionable recovery instructions.

## Testing

Covered by unit tests and end-to-end integration harness in `scripts/phase10-demo.sh`.

## Known limits

- Stdio transport active by default; HTTP streamable transport available as an opt-in mode.
