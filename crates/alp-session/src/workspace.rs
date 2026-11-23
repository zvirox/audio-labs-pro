//! Versioned workspace persistence.
//!
//! The file holds theme, Reduce motion, glass mode and the panel geometries. It
//! is written atomically through `alp-core`, so a crash during an edit or layout
//! change cannot corrupt the last good configuration.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use alp_core::{AppDirs, CoreError, write_atomic};
use serde::{Deserialize, Serialize};

use crate::panels::PanelState;

/// Current workspace schema.
pub const LAYOUT_VERSION: u32 = 1;

/// Everything restored when the application shell starts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistedWorkspace {
    /// Schema version.
    pub version: u32,
    /// `dark`, `light` or `system`.
    pub theme: String,
    /// Reduce-motion preference.
    pub reduce_motion: bool,
    /// `compositor_blur` or `tint_fallback`.
    pub glass: String,
    /// Panel geometry in painter-independent order.
    pub panels: Vec<PanelState>,
    /// Whether the control host should approve known client IDs automatically.
    /// Tokens are deliberately never persisted.
    #[serde(default)]
    pub approved_ai_clients: BTreeSet<String>,
    /// Whether first-run configuration wizard has been completed.
    #[serde(default)]
    pub first_run_completed: bool,
}

impl PersistedWorkspace {
    /// Build a layout with the current defaults.
    #[must_use]
    pub fn new(
        theme: impl Into<String>,
        reduce_motion: bool,
        glass: impl Into<String>,
        panels: Vec<PanelState>,
    ) -> Self {
        Self {
            version: LAYOUT_VERSION,
            theme: theme.into(),
            reduce_motion,
            glass: glass.into(),
            panels,
            approved_ai_clients: BTreeSet::new(),
            first_run_completed: false,
        }
    }

    /// Mark first-run completion flag.
    #[must_use]
    pub fn with_first_run_completed(mut self, completed: bool) -> Self {
        self.first_run_completed = completed;
        self
    }

    /// Persist client IDs approved for automatic connection approval.
    #[must_use]
    pub fn with_approved_ai_clients(mut self, clients: BTreeSet<String>) -> Self {
        self.approved_ai_clients = clients;
        self
    }

    /// Path of the workspace file.
    #[must_use]
    pub fn path(dirs: &AppDirs) -> PathBuf {
        dirs.config.join("workspace.json")
    }

    /// Load a compatible workspace, ignoring a missing, corrupt or newer file.
    #[must_use]
    pub fn load(path: &Path) -> Option<Self> {
        let bytes = std::fs::read(path).ok()?;
        let layout: Self = serde_json::from_slice(&bytes).ok()?;
        (layout.version == LAYOUT_VERSION).then_some(layout)
    }

    /// Write atomically, creating the config directory.
    pub fn save(&self, path: &Path) -> Result<(), CoreError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(self)?;
        write_atomic(path, &bytes)
    }

    /// Standard application workspace path from the system configuration directory.
    pub fn default_path() -> Result<PathBuf, String> {
        let dirs = AppDirs::discover().map_err(|e| e.to_string())?;
        Ok(Self::path(&dirs))
    }

    /// Load the persisted workspace from the default configuration path,
    /// or return a default layout with `size` bounds.
    #[must_use]
    pub fn load_or_default(size: impl Into<crate::panels::Size>) -> Self {
        if let Ok(path) = Self::default_path()
            && let Some(loaded) = Self::load(&path)
        {
            return loaded;
        }
        let pm = crate::panels::PanelManager::default_for(size);
        Self::new("dark", false, "compositor_blur", pm.panels().to_vec())
    }

    /// Save the workspace to the default configuration path.
    pub fn save_default(&self) -> Result<(), String> {
        let path = Self::default_path()?;
        self.save(&path).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::panels::{PanelManager, Size};

    fn layout() -> PersistedWorkspace {
        let manager = PanelManager::default_for(Size::new(1280.0, 800.0));
        PersistedWorkspace::new("dark", false, "tint_fallback", manager.snapshots())
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("nested").join("workspace.json");
        let expected = layout()
            .with_first_run_completed(true)
            .with_approved_ai_clients(BTreeSet::from(["approved-client".into()]));
        expected.save(&path).expect("save");
        let actual = PersistedWorkspace::load(&path).expect("load");
        assert_eq!(actual, expected);
        assert!(actual.first_run_completed);
        assert_eq!(
            actual.approved_ai_clients,
            BTreeSet::from(["approved-client".into()])
        );
    }

    #[test]
    fn legacy_workspace_defaults_first_run_to_false() {
        let json = r#"{
            "version": 1,
            "theme": "dark",
            "reduce_motion": false,
            "glass": "tint_fallback",
            "panels": []
        }"#;
        let ws: PersistedWorkspace = serde_json::from_str(json).expect("parse");
        assert!(!ws.first_run_completed);
    }

    #[test]
    fn missing_and_corrupt_files_are_ignored() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(PersistedWorkspace::load(&dir.path().join("missing.json")).is_none());
        let path = dir.path().join("bad.json");
        std::fs::write(&path, b"{not json").expect("write");
        assert!(PersistedWorkspace::load(&path).is_none());
    }

    #[test]
    fn newer_versions_are_ignored() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("workspace.json");
        let mut newer = layout();
        newer.version = LAYOUT_VERSION + 1;
        newer.save(&path).expect("save");
        assert!(PersistedWorkspace::load(&path).is_none());
    }

    #[test]
    fn path_lives_in_config_root() {
        let dirs = AppDirs::with_roots("/tmp/config", "/tmp/data", "/tmp/cache");
        assert_eq!(
            PersistedWorkspace::path(&dirs),
            PathBuf::from("/tmp/config/workspace.json")
        );
    }
}
