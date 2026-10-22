//! Versioned workspace persistence.
//!
//! The file holds theme, Reduce motion, glass mode and the eight panel
//! geometries. It is written atomically through `alp-core`, so a crash during a
//! drag cannot corrupt the last good layout.

use std::path::{Path, PathBuf};

use alp_core::{AppDirs, CoreError, write_atomic};
use serde::{Deserialize, Serialize};

use crate::panels::PanelState;

/// Current workspace schema.
pub const LAYOUT_VERSION: u32 = 1;

/// Everything restored when the shell starts.
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
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::panels::PanelManager;
    use iced::Size;

    fn layout() -> PersistedWorkspace {
        let manager = PanelManager::default_for(Size::new(1280.0, 800.0));
        PersistedWorkspace::new("dark", false, "tint_fallback", manager.snapshots())
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("nested").join("workspace.json");
        let expected = layout();
        expected.save(&path).expect("save");
        let actual = PersistedWorkspace::load(&path).expect("load");
        assert_eq!(actual, expected);
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
