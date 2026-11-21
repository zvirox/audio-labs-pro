# `alp-api` / `control`

## Purpose

Owner-only Linux transport for attaching local clients to the running desktop. Validates Unix peer credentials, per-host tokens and human approval, then forwards native calls or opaque MCP JSON-RPC to UI workers. It also carries explicit project-change subscriptions without owning project state.

## Public API

- `ControlHost::bind`, `stop`, `publish`, `id`, `socket_path` and `token`; host generations invalidate old queued work.
- `ControlRequest`: native authentication/project/track calls plus `mcp { message, limits }` and `subscribe`.
- `ControlResponse`: authentication, native project results, MCP JSON-RPC, `Subscribed`, `ProjectChanged` and stable errors.
- `ControlLimits`, `CONTROL_PROTOCOL_VERSION = 2`, and a 2 MiB `MAX_CONTROL_MESSAGE_BYTES` frame cap.
- `ControlEvent`, `ApprovalDecision`, `ProjectChange`, `peer_uid`, `write_request`, `read_response` and `read_response_buffered` (keeps partial frames across timeouts).

## Data flow

1. Binding creates an owner-only `0700` `alp` directory and `0600` socket without deleting unsafe existing paths.
2. Accepted connections validate `SO_PEERCRED` before parsing. First-message authentication validates protocol, fresh token and sanitized client ID, then consults human approval when needed.
3. Work enters `ControlEvent` channels with a one-shot response channel. Full MCP work proceeds through desktop worker dispatch; this module doesn't define tool semantics.
4. `subscribe` registers a dedicated change-feed worker with a 64-change bounded queue. `ControlHost::publish` uses nonblocking sends for already committed changes, dropping a subscriber that falls behind rather than allocating without bound.
5. Accepted sockets have 250 ms read and 500 ms write timeouts, so stop revokes waiting workers and closes feeds within bounded waits, then unlinks the owned socket.

## Allowed dependencies

- `std` Unix sockets, bounded buffered framing, filesystem metadata, channels and workers.
- `nix` peer credentials/effective UID; `uuid` random tokens.
- `serde`/`serde_json` tagged envelopes; permission and project/track summaries from this crate.

## Threading / real-time notes

Listener, request, subscription and response waiting happen on non-UI threads. Publishing only sends cloneable events through Rust channels; it cannot wait on subscriber I/O. No project/audio operation or real-time callback is performed here. The UI independently bounds request and completion draining.

## Error handling

Peer, protocol, token, approval, framing and path failures authenticate no work and produce explicit errors or closure. Exactly maximum-size frames round-trip; oversized writes are refused. Oversize native responses become bounded errors instead of being truncated. Tokens do not appear in errors/events. A slow reader is dropped when its bounded queue fills or a write times out; its worker exits at the next wait without blocking publication or revocation. Timed-out partial frames are retained by `read_response_buffered` so a fragmented response is still parsed.

## Testing

Real Unix-socket tests cover modes/credentials, auth/approval, token rotation, fragmentation across timeouts, exact frame boundaries, bounded subscriber queues, revocation and unsafe paths. `alp-ui` integration tests exercise MCP over this host, cross-client live history, operator/attached notifications, stale commits and reads, revocation and measured latency.

## Known limits

- Unix only. Client IDs remain self-declared labels; same-UID separation relies on secret tokens and GUI approval rather than OS sandboxing.
- Request calls on one connection are response-serialized. Change feeds are bounded at 64 pending events; a subscriber that cannot keep up is disconnected rather than backpressuring the UI. There is no multiplexed call correlation, global connection pool or remote transport.
- A 2 MiB result cap can require pagination or narrower arguments. No general external filesystem, model installation, settings or shell protocol is exposed.
