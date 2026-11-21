# `alp-ui` / `app` / `ai_control`

## Purpose

Embeds the owner-only `alp-api::control` host in the desktop, approves authenticated clients, routes cheap native calls directly, and commits prepared full-MCP calls into the existing `EditSession`. Publishes committed project changes with measured attached latency.

## Public API

Internal `App` methods initialize/toggle the host, poll queued events and MCP completions, resolve approval decisions, render the approval dialog and expose trusted client IDs. `AiControlState` holds channels, host/token/path, pending approvals, client workers, activity and notification metadata.

## Data flow

1. Enabling Settings → AI control binds `$XDG_RUNTIME_DIR/alp/control.sock` and creates an in-memory fresh token.
2. Socket threads send approval/request events; at most 64 event requests and 16 worker completions drain per frame.
3. Cheap native calls use `dispatch_control`. Opaque MCP envelopes use `ai_mcp` workers against saved project snapshots and return optional prepared transactions.
4. History tools operate on the live stack. Other successful edits commit exactly one `AttachedProjectCommand` only if host/tier/base revision remain valid; its undo entry carries the complete path and decoded-audio context, so undo/redo restores the correct project root and samples even when UUIDs are reused. A transaction-free response whose base revision is stale is rejected instead of being reported as current.
5. Every result carries `snapshotRevision` and `liveRevision` in `_meta`. Stop/tier change rejects obsolete completions. Always approval persists only the client ID, never the token.

## Allowed dependencies

- `alp_api::control`, project summaries and permission types.
- `alp_mcp` JSON-RPC/live response helpers and catalog metadata in integration tests.
- `ai_mcp` worker state and prepared transactions; the sole `EditSession` authority.
- Parent app reducer/workspace persistence, `alp-ui-kit` dialogs, standard queues and trusted-client synchronization.

## Threading / real-time notes

Socket I/O and heavy MCP dispatch are not on the UI thread. Native reads/core commands, completion validation, live undo/redo, snapshot-command installation and selection synchronization are UI-thread work. Waiting, decode, analysis, rendering, inference and export remain on workers; none executes in the audio callback.

## Error handling

Bind failures leave attach disabled. Stale hosts/tiers, revocation, unsaved desktop project and dispatch failures produce explicit refusals. Commit-time revision conflicts leave the current live project untouched; a stale read fails with its snapshot and live revisions instead of superseding a newer change notification. Oversize MCP results become bounded native errors. Activity distinguishes committed, successful read-only and failed calls.

## Testing

Unit tests cover native tier/revision enforcement, shared undo and attached path/audio context restoration across undo/redo. Real production-socket tests cover the full tier catalog, `track_add`, shared history undo/redo, two change subscribers' AI/operator events, measured structural-commit latency, concurrent human edit protection, stale-read rejection and late worker revocation. `alp-api::control` covers socket security, bounded subscribers and framing.

## Known limits

- Full-session attached rollback is refused; “Undo last edit” remains shared-stack undo, not AI-only rollback.
- `queueToCommitMs` measures accepted-call elapsed time through worker preparation, commit and view-model synchronization, not actual pixels/audio presentation.
- No per-connection activity indicators, flood/pipelining benchmark, background-job progress UI, generic cancellation or destructive-operation approval round trip is implemented.
