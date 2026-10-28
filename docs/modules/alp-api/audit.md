# `alp-api` / `audit`

## Purpose

Provides persistent JSONL append-only audit logging for tool executions and project mutations.

## Public API

- `AuditEntry`: structured log record.
- `AuditLogger::new(path) -> Result<Self, ApiError>`
- `AuditLogger::record(&self, entry: &AuditEntry) -> Result<(), ApiError>`

## Data flow

1. Tool invocations construct an `AuditEntry` with client identity, tool name, argument digest, permission tier, and revisions.
2. Appends JSON line atomically to the audit log file.

## Allowed dependencies

- `std::fs::OpenOptions`
- `std::io::Write`
- `serde::{Deserialize, Serialize}`
- `crate::error::ApiError`
- `crate::types::PermissionTier`

## Threading / real-time notes

File append operations occur during tool execution on worker/RPC threads; never in the real-time audio thread.

## Error handling

Returns `ApiError::Io` or `ApiError::Json` upon write failure.

## Testing

Verified with unit tests creating and reading back audit logs in temporary directories.

## Known limits

- Invocations log argument digests and lengths, never raw audio buffers or full transcripts.
