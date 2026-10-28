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

## Data flow

1. AI client or script initializes `ApiSession` within an allowed workspace root.
2. Checks permissions against `PermissionTier`.
3. Dispatches typed operations to `alp-core`, `alp-media`, `alp-dsp`, `alp-voice`, and `alp-engine`.
4. Records structured audit logs and updates revision counters.

## Allowed dependencies

- `alp-core`
- `alp-media`
- `alp-dsp`
- `alp-voice`
- `alp-engine`

## Threading / real-time notes

Operates out of band from the real-time audio callback. Audio rendering executes asynchronously in worker jobs.

## Error handling

Returns structured `ApiError` with clear diagnostics and suggested recovery actions.

## Testing

Unit tests in crate modules; integration tests via `alp-mcp` and `scripts/phase10-demo.sh`.

## Known limits

- Single writer per project session.
