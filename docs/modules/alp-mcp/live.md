# `alp-mcp` / `live`

## Purpose

Small response adapters where authoritative desktop state must replace values from a worker-local scratch `ApiSession`, without duplicating the MCP dispatcher.

## Public API

Internal library helpers expose tool-name extraction, live project-summary patching, failed-commit results, and history list/undo/redo responses using the ordinary MCP `ToolCallResult` contract.

## Data flow

The attached worker rebases scratch state and runs `McpServer`. These helpers patch `project_info` and `alp://project/active` with the GUI summary. The UI maps scratch history requests to `EditSession` and formats their real outcomes through the same helpers.

## Allowed dependencies

- `alp_api` summary, history and pagination types.
- `serde_json` and this crate's protocol response types.

## Threading / real-time notes

All functions are synchronous data transformations. They do no I/O, own no project state and do not dispatch commands. The desktop calls history response constructors only after its UI-thread live-stack operation.

## Error handling

Commit helpers always produce a domain-level `isError: true` response. Pagination failures preserve the actionable API error. JSON serialization uses non-failing protocol structures or a serializable summary fallback.

## Testing

The full attached integration test in `alp-ui::app::ai_control` checks real `history_list`, `history_undo` and `history_redo`, plus live catalog and project operations. Shared MCP result semantics are covered by headless suite tests.

## Known limits

There is no generalized alternate backend for shared dispatch; only project summaries and history are adapted here. Attached full-session rollback is deliberately refused to avoid discarding operator edits from an obsolete scratch checkpoint.
