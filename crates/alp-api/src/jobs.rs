//! Asynchronous background job manager and task tracking.

use alp_core::now_ms;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::error::ApiError;

/// Status state of a background job.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum JobState {
    /// Queued awaiting execution.
    Queued,
    /// Currently running with normalized progress (0.0 to 1.0).
    Running { progress: f32, detail: String },
    /// Completed successfully with a result payload.
    Completed { result: String },
    /// Failed with an error message.
    Failed { error: String },
    /// Cancelled before completion.
    Cancelled,
}

/// Description of a background job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobInfo {
    /// Unique job ID (e.g. `job_12345`).
    pub id: String,
    /// Human-readable job label.
    pub label: String,
    /// Milliseconds since UNIX epoch when submitted.
    pub created_at_ms: u64,
    /// Current job execution state.
    pub state: JobState,
}

#[derive(Debug)]
struct JobRecord {
    info: JobInfo,
    cancelled: bool,
}

/// Thread-safe manager for background jobs.
#[derive(Debug, Clone, Default)]
pub struct JobManager {
    jobs: Arc<Mutex<HashMap<String, JobRecord>>>,
}

impl JobManager {
    /// Create a new job manager.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Submit a job and return its `JobId`.
    pub fn submit(&self, label: impl Into<String>) -> String {
        // UUID-based ids: submissions within the same millisecond must not
        // overwrite each other in the job map.
        let id = format!("job_{}", uuid::Uuid::new_v4().simple());
        let info = JobInfo {
            id: id.clone(),
            label: label.into(),
            created_at_ms: now_ms(),
            state: JobState::Queued,
        };
        let mut map = self.jobs.lock().unwrap();
        map.insert(
            id.clone(),
            JobRecord {
                info,
                cancelled: false,
            },
        );
        id
    }

    /// Update progress for a running job.
    pub fn update_progress(&self, id: &str, progress: f32, detail: impl Into<String>) {
        let mut map = self.jobs.lock().unwrap();
        if let Some(record) = map.get_mut(id)
            && !record.cancelled
        {
            record.info.state = JobState::Running {
                progress: progress.clamp(0.0, 1.0),
                detail: detail.into(),
            };
        }
    }

    /// Mark a job as completed.
    pub fn complete(&self, id: &str, result: impl Into<String>) {
        let mut map = self.jobs.lock().unwrap();
        if let Some(record) = map.get_mut(id) {
            record.info.state = JobState::Completed {
                result: result.into(),
            };
        }
    }

    /// Mark a job as failed.
    pub fn fail(&self, id: &str, error: impl Into<String>) {
        let mut map = self.jobs.lock().unwrap();
        if let Some(record) = map.get_mut(id) {
            record.info.state = JobState::Failed {
                error: error.into(),
            };
        }
    }

    /// Cancel a job.
    pub fn cancel(&self, id: &str) -> Result<(), ApiError> {
        let mut map = self.jobs.lock().unwrap();
        let record = map
            .get_mut(id)
            .ok_or_else(|| ApiError::NotFound(format!("job '{id}' not found")))?;
        record.cancelled = true;
        record.info.state = JobState::Cancelled;
        Ok(())
    }

    /// Check whether a job was cancelled.
    #[must_use]
    pub fn is_cancelled(&self, id: &str) -> bool {
        self.jobs
            .lock()
            .unwrap()
            .get(id)
            .is_some_and(|r| r.cancelled)
    }

    /// Get current status info for a job.
    pub fn get_status(&self, id: &str) -> Result<JobInfo, ApiError> {
        let map = self.jobs.lock().unwrap();
        let record = map
            .get(id)
            .ok_or_else(|| ApiError::NotFound(format!("job '{id}' not found")))?;
        Ok(record.info.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_lifecycle() {
        let manager = JobManager::new();
        let job_id = manager.submit("Master render");
        let initial = manager.get_status(&job_id).expect("status");
        assert!(matches!(initial.state, JobState::Queued));

        manager.update_progress(&job_id, 0.5, "Rendering audio");
        let running = manager.get_status(&job_id).expect("status");
        if let JobState::Running { progress, .. } = running.state {
            assert!((progress - 0.5).abs() < 1e-4);
        } else {
            panic!("expected Running");
        }

        manager.complete(&job_id, "render.wav");
        let completed = manager.get_status(&job_id).expect("status");
        assert!(matches!(completed.state, JobState::Completed { .. }));
    }

    #[test]
    fn job_ids_are_unique_for_rapid_submissions() {
        let manager = JobManager::new();
        let ids: Vec<String> = (0..300)
            .map(|i| manager.submit(format!("job {i}")))
            .collect();
        let unique: std::collections::HashSet<&String> = ids.iter().collect();
        assert_eq!(
            unique.len(),
            ids.len(),
            "jobs submitted within the same millisecond must not collide"
        );
    }
}
