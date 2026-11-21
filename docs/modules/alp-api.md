# `alp-api`

## Purpose

The automation service layer for Audio Labs Pro. Provides typed, permission-tiered, undoable project mutations, media management, DSP processing, speech synthesis, transcription, and mastering export for external AI agents, MCP clients, and headless scripts.

## Public API

- `ApiSession`: active automation coordinator.
- `WorkspaceSandbox`: filesystem sandboxing.
- `AuditLogger`: append-only JSONL event logging.
- `CheckpointManager`: session rollback snapshots.
- `JobManager`: async task execution.
- `PermissionTier`: `Observe`, `Edit`, `Export` security tiers.
- `control::{ControlHost, ControlRequest, ControlResponse, ControlEvent}` (Linux):
  owner-only desktop attach transport, admission, token handshake and bounded
  framing. Socket threads enqueue work; the GUI owns execution.
- `nix`: safe `SO_PEERCRED` and effective-UID queries for the control boundary.

## Data flow

1. AI client or script initializes `ApiSession` within an allowed workspace root.
2. Checks permissions against `PermissionTier`.
3. Dispatches typed operations to `alp-core`, `alp-media`, `alp-dsp`, `alp-voice`, and `alp-engine`.
4. Records structured audit logs and updates revision counters.

For desktop attach, `ControlHost` only validates the Unix peer, token and human
approval. Approved legacy calls or JSON-RPC MCP envelopes travel as
`ControlEvent`s to `alp-ui`. Desktop workers use `AttachedProjectState` as
revision-tagged scratch execution state, but only the GUI's existing
`EditSession` can acquire the resulting authoritative project and undo history.

## Allowed dependencies

- `alp-core`
- `alp-media`
- `alp-dsp`
- `alp-voice`
- `alp-engine`

Linux control transport additionally uses `nix` with only `socket` and `user`
features; the workspace's `unsafe_code = "deny"` lint remains in force.

## Threading / real-time notes

Operates out of band from the real-time audio callback. The owner-only control
socket reads and waits on background threads; its channels are drained without
blocking on the UI thread. Full attached MCP decoding, analysis, rendering and
generation run on desktop workers, outside this transport module.

## Error handling

Returns structured `ApiError` with clear diagnostics and suggested recovery actions.

## Testing

Unit tests in crate modules; integration tests via `alp-mcp` and
`scripts/phase10-demo.sh`. Control tests exercise real Unix sockets, peer
credentials trust, token and approval boundaries, revocation, fragmented I/O,
and queue replies. `alp-ui` adds production-socket full-MCP, shared history,
media reconciliation, conflict, revocation and live-notification integration.

## Known limits

- Single writer per project session.
- Full attached MCP uses opaque native envelopes and desktop worker snapshots.
  Scratch `ApiSession`s are not project authorities. No remote/network transport.
