# `alp-mcp` / `server`

## Purpose

Transport-level MCP server: routes JSON-RPC methods, owns the three boundary
limits (payload cap, rate limit, advisory call budget), applies the
optimistic-concurrency guard, and serves the resource and prompt surfaces.
Tool dispatch itself lives in [`dispatch`](dispatch.md), so the envelope rules
and the tool mapping cannot drift apart — every tool is limited the same way,
and a new tool cannot bypass the guard by forgetting to restate it.

## Public API

- `McpServer::new(session: Arc<Mutex<ApiSession>>, tier: PermissionTier) -> Self`
- `McpServer::with_limits(limits: Limits) -> Self` — replaces the boundary
  limits (used by `main.rs` from the CLI flags and by tests).
- `McpServer::handle_request(&self, req: JsonRpcRequest) -> JsonRpcResponse`

## Data flow

1. `main.rs` reads a line, checks it against `Limits::max_payload_bytes`, then
   parses it as JSON-RPC.
2. `handle_request` routes by method: `initialize`, `server/discover`,
   `tools/list`, `tools/call`, `resources/*`, `prompts/*`.
3. `tools/list` filters `tool_defs::tools_for_tier` by tier and adds
   `ttlMs`/`cacheScope` hints. `server/discover` publishes the spec revision,
   the configured limits (including `remainingInWindow`) and the pagination
   contract.
4. `tools/call`: admit through the sliding-window `RateLimiter` *before* taking
   the session lock (a flooding client must not starve legitimate calls of the
   mutex); lock the session; `stage_arguments`; apply the `expected_revision`
   guard for non-read-only tools; measure the call, delegate to
   `dispatch::dispatch`, and append an honest over-budget note when
   `check_elapsed` reports an overrun.

## Allowed dependencies

- `std::sync::{Arc, Mutex}`
- `serde_json::{Value, json}`
- `alp_api::{ApiSession, PermissionTier}`
- `crate::protocol`, `crate::tool_defs`, `crate::dispatch`, `crate::limits`

## Threading / real-time notes

One session mutex serialises all calls, so project revisions are strictly
ordered. The rate limiter is behind its own mutex and never held across a tool
call. Nothing here runs on the audio callback path.

## Error handling

Unknown methods produce JSON-RPC `-32601`; a malformed cursor, zero limit or
rate-limit refusal becomes an `isError: true` tool result naming the limit and
the recovery. The payload cap answers with a failure result rather than a parse
error because the line may not even be parseable. A concurrency conflict is a
tool failure that names `project_info` as the recovery call; reads never fail
on a stale guard, because the read is what the agent needs to recover.

## Testing

30 tests in `crates/alp-mcp/src/main.rs` exercise this module through
`handle_request`, including tier visibility, discovery, pagination, the
concurrency guard, and the rate limit.

## Known limits

- Single session per server instance; one client per process.
- The call budget is advisory: calls run synchronously under the session
  mutex, so an over-budget call reports what actually happened instead of
  being cancelled.
