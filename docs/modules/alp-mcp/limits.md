# `alp-mcp::limits`

## Purpose

Boundary limits for the MCP server process: request payload cap, advisory call
time budget, and a sliding-window rate limit. They exist so one attached agent
cannot stall or flood the host, and so every rejection names the limit and how
to recover — a silent throttle reads to an agent as a broken tool.

## Public API

- `Limits` — the three configured boundaries (`max_payload_bytes`,
  `call_timeout`, `requests_per_window`, `window`), with generous defaults
  (1 MiB, 120 s, 600 requests / 60 s).
- `Limits::check_payload(bytes) -> Result<(), LimitError>` — rejects an
  over-cap request line before parsing.
- `Limits::check_elapsed(duration) -> Result<(), LimitError>` — reports an
  over-budget call.
- `RateLimiter::new(limits)` / `admit(now) -> Result<(), LimitError>` — counts
  one request or refuses it; prunes stamps older than the window first so the
  budget refills continuously instead of resetting on a fixed boundary.
- `RateLimiter::used()` / `remaining(now)` — window counters surfaced through
  `server/discover` as `limits.usedInWindow` / `limits.remainingInWindow`.
- `TimedOutcome::record(value, elapsed)` — the measured duration of a completed
  call, used to append the honest over-budget note.
- `LimitError` — `PayloadTooLarge`, `Timeout`, `RateLimited`, each message
  naming the limit and the recovery.

## Data flow

1. `main.rs` builds `Limits` from CLI flags and hands them to
   `McpServer::with_limits`.
2. The read loop calls `check_payload` on every incoming line before JSON
   parsing.
3. `handle_tool_call` admits the call through the `RateLimiter` *before* taking
   the session lock, so a flooding client cannot starve legitimate calls of the
   mutex.
4. After dispatch, `check_elapsed` on the measured duration appends an
   over-budget note to the result.

## Allowed dependencies

- `std::time::{Duration, Instant}`
- `thiserror`

No other crates.

## Threading / real-time notes

The limiter lives behind a `Mutex` inside `McpServer` and runs on the server
thread; it never touches audio state. The window is wall-clock (`Instant`), so
it is monotonic and unaffected by wall-clock adjustments.

## Error handling

`LimitError` values are converted to tool-level failures (never JSON-RPC
protocol errors), so a limited client sees a normal `isError` result with the
cause and recovery in the text. The rate limit refuses the call it cannot
serve; the payload cap answers with a failure result instead of a parse error,
because the line may not even be parseable JSON.

## Testing

Five unit tests in `limits.rs`:

- the payload cap accepts the exact limit and rejects one byte over, naming the
  budget in the message;
- the sliding window admits the budget, refuses the next request, and refills
  as old stamps age out;
- the timeout path distinguishes within-budget from over-budget calls while
  still returning the real result;
- `remaining` shrinks as requests are admitted and recovers after the window.

## Known limits

- The rate limit is per process. `alp-mcp` serves one client per process, so
  "per client" means per process today; the limiter is ready to be keyed by
  client label if the server later multiplexes sessions.
- The time budget is advisory: calls run synchronously on the server thread
  (the session is a single-writer `Mutex`), so a wedged call cannot be
  interrupted. An over-budget call reports what actually happened — its real
  result plus a note — instead of fabricating a fast success.
