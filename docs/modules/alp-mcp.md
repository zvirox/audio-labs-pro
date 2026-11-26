# `alp-mcp`

## Purpose

The local Model Context Protocol server for Audio Labs Pro, both as a standalone headless project session and as an `--attach` proxy into the running desktop. AI clients can inspect projects, perform undoable edits, analyze audio, generate approved voice content, and export within the existing safety boundaries.

## Public API

- Binary: `alp-mcp --workspace <path> [--tier observe|edit|export]` for headless operation.
- Attached binary: `ALP_CONTROL_TOKEN=... alp-mcp --attach`, with no disk-project initialization.
- Library: `McpServer`, JSON-RPC/tool result types, tool catalog, protocol limits and live response adapters.
- Standard JSON-RPC methods include `initialize`, `server/discover`, tools, resources and prompts. Attached resource subscriptions receive live change notifications.

## Data flow

Headless mode creates/opens its explicitly selected `ProjectPaths`, workspace sandbox, audit and `ApiSession`, then dispatches line-delimited stdio JSON-RPC through `McpServer`. Attach mode authenticates to the desktop before any disk-project initialization. It forwards complete envelopes to `alp-ui`; desktop workers rebase scratch `ApiSession`s onto current GUI snapshots and reuse `McpServer`. Successful mutations commit only into the live `EditSession`. Headless disk projects and the desktop never silently share authority.

## Allowed dependencies

- `alp-core`
- `alp-api`
- CLI/protocol support: `clap`, `serde`, `serde_json`, `anyhow`, `thiserror`, `tracing`.

The desktop may depend on this library; the MCP crate never depends on the UI, avoiding an ownership cycle. `scripts/check-deps.sh` enforces these directions.

## Threading / real-time notes

Headless calls use the API's mutex-protected session. Attached calls run on per-client desktop workers, including media decode, rendering and voice work. Only revision-checked live commits and cheap view synchronization occur on the UI thread. A separate proxy thread forwards change-feed events without blocking request execution on socket I/O.

## Error handling

Malformed JSON returns standard JSON-RPC errors. Tool-domain failures return `isError: true` with recovery guidance. Attached revision conflicts and revocation preserve authoritative live state. Payload/result overflow produces bounded errors. Cloning, model installation, settings, generic file/shell access and unsafe paths remain unavailable.

## Testing

`cargo test -p alp-mcp` covers the shared catalog, schemas, dispatch, tools, limits and headless workflows. Live attach tests exercise the shared dispatcher against attached workers, including history, media-source reconciliation, conflict refusal and revocation. `bash scripts/mcp-eval.sh` runs the 8 legacy suites plus 25 named Phase 18 scenarios over real stdio and writes `target/mcp_eval_results.json`.

## Known limits

- Stdio is implemented; Streamable HTTP is not.
- Attach currently serializes calls per connection and does not implement general cancellation or request pipelining.
- MCP initialization/cancellation behavior, true per-call timeouts and bounded concurrent jobs remain incomplete.
- No `outputSchema` or ~20 KB text cap yet; the 25-scenario evaluation/convergence suite lives in `scripts/mcp_eval_scenarios.py`.
- Latency metadata measures request-to-live-commit/view-model readiness, not actual pixel/audio presentation.
