//! Project session checkpointing and rollback manager.

use alp_core::{Project, now_ms};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::ApiError;
/// An immutable saved state snapshot of a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Checkpoint identifier (e.g. `chk_1a2b3c`).
    pub id: String,
    /// Human-readable label or description.
    pub label: String,
    /// Milliseconds since UNIX epoch when checkpoint was captured.
    pub created_at_ms: u64,
    /// Client or user who created this checkpoint.
    pub creator: String,
    /// Deep copy of the project state.
    pub project_snapshot: Project,
}

/// In-memory manager for project checkpoints.
#[derive(Debug, Default, Clone)]
pub struct CheckpointManager {
    checkpoints: HashMap<String, Checkpoint>,
    initial_session_checkpoint: Option<String>,
}

impl CheckpointManager {
    /// Create an empty checkpoint manager.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Capture a checkpoint of the current project state.
    pub fn create_checkpoint(
        &mut self,
        project: &Project,
        label: impl Into<String>,
        creator: impl Into<String>,
    ) -> String {
        // UUID-based ids: back-to-back checkpoints can land in the same
        // millisecond, and `now_ms`-derived ids would silently overwrite the
        // session-start checkpoint that rollback depends on.
        let id = format!("chk_{}", uuid::Uuid::new_v4().simple());
        let cp = Checkpoint {
            id: id.clone(),
            label: label.into(),
            created_at_ms: now_ms(),
            creator: creator.into(),
            project_snapshot: project.clone(),
        };
        if self.initial_session_checkpoint.is_none() {
            self.initial_session_checkpoint = Some(id.clone());
        }
        self.checkpoints.insert(id.clone(), cp);
        id
    }

    /// Retrieve a saved checkpoint by ID.
    #[must_use]
    pub fn get_checkpoint(&self, id: &str) -> Option<&Checkpoint> {
        self.checkpoints.get(id)
    }

    /// List all checkpoints sorted by timestamp.
    #[must_use]
    pub fn list_checkpoints(&self) -> Vec<Checkpoint> {
        let mut list: Vec<Checkpoint> = self.checkpoints.values().cloned().collect();
        list.sort_by_key(|c| c.created_at_ms);
        list
    }

    /// ID of the initial checkpoint captured when this session began.
    #[must_use]
    pub fn initial_checkpoint_id(&self) -> Option<&str> {
        self.initial_session_checkpoint.as_deref()
    }

    /// Restore project to a specific checkpoint.
    pub fn restore_checkpoint(&self, id: &str, target: &mut Project) -> Result<(), ApiError> {
        let cp = self
            .checkpoints
            .get(id)
            .ok_or_else(|| ApiError::NotFound(format!("checkpoint '{id}' not found")))?;
        *target = cp.project_snapshot.clone();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_capture_and_restore() {
        let mut project = Project::new("Before Edit").expect("project");
        let mut manager = CheckpointManager::new();

        let id = manager.create_checkpoint(&project, "Initial", "test-agent");
        assert_eq!(manager.initial_checkpoint_id(), Some(id.as_str()));

        // Mutate project
        project.name = "After Edit".into();
        assert_eq!(project.name, "After Edit");

        // Restore
        manager
            .restore_checkpoint(&id, &mut project)
            .expect("restore");
        assert_eq!(project.name, "Before Edit");
    }
}
