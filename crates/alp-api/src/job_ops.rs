//! Background job operations for `ApiSession`.

use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::jobs::JobState;
use crate::session::ApiSession;
use crate::types::PermissionTier;

/// Status report for a background job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStatusReport {
    /// Job ID.
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// Current state.
    pub state: String,
    /// Progress percentage (0.0 to 1.0).
    pub progress: f32,
    /// Human-readable progress detail.
    pub detail: String,
}

impl ApiSession {
    /// Get current status of a background job.
    pub fn job_status(&self, job_id: &str) -> Result<JobStatusReport, ApiError> {
        self.check_permission(PermissionTier::Observe)?;
        let job = self.jobs().get_status(job_id)?;
        let (state_str, progress) = match &job.state {
            JobState::Queued => ("queued".to_string(), 0.0),
            JobState::Running { progress, .. } => ("running".to_string(), *progress),
            JobState::Completed { .. } => ("completed".to_string(), 1.0),
            JobState::Failed { .. } => ("failed".to_string(), 0.0),
            JobState::Cancelled => ("cancelled".to_string(), 0.0),
        };
        Ok(JobStatusReport {
            id: job.id.clone(),
            label: job.label.clone(),
            state: state_str,
            progress,
            detail: job.label.clone(),
        })
    }

    /// Cancel a running background job.
    pub fn job_cancel(&mut self, job_id: &str) -> Result<(), ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        self.jobs().cancel(job_id)?;
        Ok(())
    }
}
