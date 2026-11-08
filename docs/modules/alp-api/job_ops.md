# `alp-api` / `job_ops`

## Purpose

Provides background job inspection and cancellation operations for long-running asynchronous tasks (renders, downloads, batch exports) in `ApiSession`.

## Public API

- `JobStatusReport`: job ID, label, state ("queued", "running", "completed", "failed", "cancelled"), progress (0.0 to 1.0), detail.
- `ApiSession::job_status(&self, job_id: &str) -> Result<JobStatusReport, ApiError>`
- `ApiSession::job_cancel(&mut self, job_id: &str) -> Result<(), ApiError>`

## Data flow

1. Verifies caller permissions (`PermissionTier::Observe` for queries, `PermissionTier::Edit` for cancellations).
2. Delegates to the underlying `JobManager` tracked in `ApiSession`.
3. Reads atomic progress and state or signals cancellation tokens to running threads.
4. Returns formatted status report.

## Allowed dependencies

- `serde::{Deserialize, Serialize}`
- `crate::error::ApiError`
- `crate::session::ApiSession`
- `crate::types::PermissionTier`
- `crate::jobs::JobState`

## Threading / real-time notes

Interacts with job coordination locks; non-blocking to real-time audio threads.

## Error handling

Returns `ApiError::NotFound` if the requested `job_id` does not exist; `ApiError::PermissionDenied` if permissions are insufficient.

## Testing

Tested by job polling and cancellation unit tests in `alp-api`.

## Known limits

- Cancelled jobs rely on worker task cooperative polling to clean up transient files.
