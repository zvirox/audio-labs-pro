# `alp-ui` / `app` / `ai_mcp`

## Purpose

Worker execution and preparation for the full attached MCP catalog. It keeps a persistent per-client server while rebasing an explicitly scratch `ApiSession` onto each current GUI snapshot.

## Public API

`AttachedMcpState` accepts native MCP calls, returns completions to the UI, tracks measured commit timings, and drops client worker channels on tier change or revocation. `PreparedTransaction` distinguishes live edits from save-only state. These APIs are visible only to the parent app module.

## Data flow

1. The UI supplies one saved `AttachedProjectState` and summary.
2. A per-client worker clamps limits, creates/reuses `McpServer`, rebases the scratch session, and dispatches.
3. Read/failure/history responses return without a project transaction.
4. A changed project has all needed source audio decoded on the worker before an edit transaction is emitted.
5. `ai_control` checks host generation, tier and base revision, then commits to the one `EditSession` or returns a conflict.

## Allowed dependencies

- `alp_mcp` for the shared server, protocol, limits and live-response adapters.
- `alp_api` for scratch state, permission, sandbox and audit.
- `alp_media` and `alp_engine` for worker decode and source audio caches.
- Standard threads, channels, synchronization and monotonic timing.

## Threading / real-time notes

All decode, analysis, rendering, generation, export and scratch dispatch run on attached workers. The UI drains up to 16 completions per frame without waiting. Only cheap validation, command installation and view synchronization happen at commit. This module never touches the real-time audio callback.

## Error handling

Invalid envelopes, excessive inputs, tier/limit changes, initialization errors, decode failures and failed tool calls do not produce edit transactions. Newer host/tier state rejects stale completions. A live revision conflict prevents the prepared project from replacing operator work.

## Testing

The integration tests in `ai_control.rs` use the actual Unix host, worker and GUI session. They cover the complete tier-filtered catalog, shared undo/redo, one undo step per attach edit, stale worker rejection, revocation, operator/AI notifications and structural-edit latency.

## Known limits

Every changed project currently decodes its sources to reconcile playback audio, so the ≤200 ms evidence applies only to the tested cache-ready structural setup. `rollback_session` is refused in attached mode. Calls are sequential per client worker; there is no pipelining or general cancellation. Bitmap/pixel presentation is not measured by `queueToCommitMs`.
