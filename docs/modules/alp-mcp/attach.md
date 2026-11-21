# `alp-mcp` / `attach`

## Purpose

Native JSON-RPC stdio proxy for attaching to an already-running, approved desktop control host. It reuses the desktop MCP dispatcher instead of opening a project or creating a second undo history.

## Public API

The CLI selects this module with `alp-mcp --attach`. `--control-socket`, `--client-id`, the normal boundary limit flags, `ALP_CONTROL_TOKEN`, and `XDG_RUNTIME_DIR` control connection and limits. The entry point is internal to the binary.

## Data flow

1. Resolve the owner-only socket and obtain a fresh token from the environment.
2. Authenticate a request connection and a separate authenticated change-feed connection.
3. Parse line-delimited JSON-RPC requests from stdin and forward them in native `mcp` envelopes.
4. Return the desktop JSON-RPC result or correlate a native refusal with the incoming JSON-RPC ID.
5. Gate desktop change events on `resources/subscribe`; retain partial frames across socket timeouts and emit standard resource updates plus the detailed ALP change extension.

## Allowed dependencies

- `alp_api::control` for native authentication, framing and subscriptions.
- The parent library's JSON-RPC types and `serde_json`.
- `anyhow`, `tracing`, Unix sockets, standard I/O and channels.

## Threading / real-time notes

The stdin loop handles calls synchronously per connection. A second thread reads change notifications and locks stdout only while writing a complete line. Socket waiting occurs here and in the desktop workers, never on the UI or audio threads.

## Error handling

Missing endpoints/tokens, authentication refusal, EOF, malformed JSON and oversized requests produce explicit errors or JSON-RPC errors. Oversized responses become bounded correlated errors, never truncated content. Subscription to an unsupported URI fails with invalid parameters; a timed-out partial notification frame is preserved instead of corrupting the next event.

## Testing

`cargo check -p alp-mcp` compiles this binary transport. The production-socket integration tests in `alp-ui` exercise the corresponding native host and shared MCP dispatcher, including edits, history, notifications, conflicts and revocation.

## Known limits

Calls do not pipeline and cancellation is not implemented. The token must be supplied afresh for every enabled desktop host. Direct end-to-end tests of the stdio process are not yet separate from the native-host MCP integration tests.
