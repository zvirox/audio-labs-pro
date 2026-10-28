# `alp-api` / `jobs`

## Purpose

Tracks asynchronous background tasks (such as speech generation, transcription, and master audio rendering) with normalized progress and cancellation.

## Public API

- `JobState`: `Queued`, `Running`, `Completed`, `Failed`, `Cancelled`.
- `JobInfo`: metadata and current state.
- `JobManager::new() -> Self`
- `JobManager::submit(&self, label) -> String`
- `JobManager::update_progress(&self, id, progress, detail)`
- `JobManager::complete(&self, id, result)`
- `JobManager::cancel(&self, id) -> Result<(), ApiError>`

## Data flow

1. Long-running tasks register with `JobManager::submit` receiving a unique handle (`job_...`).
2. Workers periodically update normalized progress ($0.0 \dots 1.0$) and status messages.
3. Callers inspect progress or issue cancellations.

## Allowed dependencies

- `std::collections::HashMap`
- `std::sync::{Arc, Mutex}`
- `serde::{Deserialize, Serialize}`
- `crate::error::ApiError`

## Threading / real-time notes

Thread-safe using internal mutex synchronization across worker threads.

## Error handling

Cancelling or querying missing jobs returns `ApiError::NotFound`.

## Testing

Covered by unit tests verifying the complete queued $\to$ running $\to$ completed lifecycle.

## Known limits

- Job history is maintained in-memory.
