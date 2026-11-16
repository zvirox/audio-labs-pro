# `alp-mcp` / `protocol`

## Purpose

Defines JSON-RPC 2.0 and Model Context Protocol envelope types, tool schemas,
tool behavioural metadata, and content responses.

## Public API

- `JsonRpcRequest` / `JsonRpcResponse` / `JsonRpcError` — JSON-RPC envelopes.
- `McpTool` — one `tools/list` entry. Serialization adds `title` and
  `annotations` looked up from `tool_defs::metadata_for`, so a tool's
  human-facing title and behaviour hints are declared once, next to its
  schema, and cannot be forgotten on a new tool.
- `ToolAnnotations` — the four MCP hints (`readOnlyHint`, `destructiveHint`,
  `idempotentHint`, `openWorldHint`) with three named presets: `read_only`,
  `additive`, `destructive`. `to_json` renders the wire object.
- `ToolMetadata` — `{ title: String, annotations: ToolAnnotations }`.
- `ToolCallResult` — text + structured content. `success(summary, json)` for
  real results, `failure(message)` for explained failures, and `with_note(note)`
  to append a note (e.g. an over-budget report) while keeping the structured
  payload.

## Data flow

1. Deserializes client requests from JSON strings.
2. `tools/list` serializes each `McpTool` with its `title`/`annotations`.
3. Tool call results are formatted with text summaries and structured JSON
   representations; `with_note` is used by the server when a call exceeds its
   advisory time budget, so the overrun is visible without discarding the real
   result.

## Allowed dependencies

- `serde::{Deserialize, Serialize}`
- `serde_json::{Value, json}`
- `crate::tool_defs::metadata_for` (for the title/annotations lookup)

## Threading / real-time notes

Pure serializable message envelopes with no external state.

## Error handling

Provides constructor helpers for protocol errors and tool-level failure
payloads. `failure` never fabricates structured content — an error result has
`isError: true` and an explanation, no fake payload.

## Testing

Covered by serialization round-trips and the metadata parity tests in
`alp-mcp` (`every_advertised_tool_declares_a_title_and_annotations`).

## Known limits

- Conforms to standard JSON-RPC 2.0 conventions.
- No `outputSchema` on `McpTool` yet.
- Annotations are static contract hints, not per-invocation guarantees.
