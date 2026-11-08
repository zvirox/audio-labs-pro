//! Project and history lifecycle operations for `ApiSession`.

use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::session::ApiSession;
use crate::types::PermissionTier;
use alp_core::{Project, ProjectPaths};

/// Summary item in project list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectListItem {
    /// Project name.
    pub name: String,
    /// Path to project directory or file.
    pub path: String,
    /// Sample rate.
    pub sample_rate: u32,
    /// Track count.
    pub track_count: usize,
}

/// Description of an undo/redo stack entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Index in stack.
    pub index: usize,
    /// Name or description of command.
    pub description: String,
    /// Whether this entry can be undone.
    pub is_undo: bool,
}

impl ApiSession {
    /// List known project files/directories in workspace sandbox.
    pub fn project_list(&self) -> Result<Vec<ProjectListItem>, ApiError> {
        self.check_permission(PermissionTier::Observe)?;
        let list = vec![ProjectListItem {
            name: self.project.name.clone(),
            path: self.paths.root().display().to_string(),
            sample_rate: self.project.sample_rate,
            track_count: self.project.tracks.len(),
        }];

        Ok(list)
    }

    /// Create and switch to a new project in the workspace.
    pub fn project_create(
        &mut self,
        name: &str,
        sample_rate: Option<u32>,
    ) -> Result<String, ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        let rate = sample_rate.unwrap_or(48_000);
        let project = Project::new_with_rate(name, rate)
            .map_err(|e| ApiError::Validation(format!("project creation failed: {e}")))?;

        self.project = project;
        self.save_project()?;
        self.record_audit("project_create", "ok", self.revision());
        Ok(self.project.id.to_string())
    }

    /// Open an existing project from disk.
    pub fn project_open(&mut self, path: &str) -> Result<String, ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        let validated = self.sandbox.validate_path(path)?;
        let paths = ProjectPaths::new(&validated);
        let project = paths
            .load()
            .map_err(|e| ApiError::Validation(format!("failed to load project: {e}")))?;

        self.project = project;
        self.paths = paths;
        self.record_audit("project_open", "ok", self.revision());
        Ok(self.project.id.to_string())
    }

    /// Close current project and revert to an empty draft project.
    pub fn project_close(&mut self) -> Result<(), ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        let fresh =
            Project::new("Untitled Project").map_err(|e| ApiError::Validation(e.to_string()))?;
        self.project = fresh;
        self.record_audit("project_close", "ok", self.revision());
        Ok(())
    }

    /// Inspect undo/redo history stack entries.
    pub fn history_list(&self) -> Result<Vec<HistoryEntry>, ApiError> {
        self.check_permission(PermissionTier::Observe)?;
        let mut entries = Vec::new();
        // Since CommandStack doesn't expose internal strings, list available actions
        let info = self.project_info()?;
        for i in 0..info.undo_steps {
            entries.push(HistoryEntry {
                index: i,
                description: format!("Undoable edit step #{}", i + 1),
                is_undo: true,
            });
        }
        for i in 0..info.redo_steps {
            entries.push(HistoryEntry {
                index: i,
                description: format!("Redoable edit step #{}", i + 1),
                is_undo: false,
            });
        }
        Ok(entries)
    }
}
