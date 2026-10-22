//! Project directory layout, atomic saving and loading.
//!
//! A project is a directory:
//!
//! ```text
//! my-project.alp/
//!   project.json
//!   media/
//!   peaks/
//!   renders/
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use crate::CoreError;
use crate::files::write_atomic;
use crate::project::Project;
use crate::schema;

/// Filesystem layout of one project directory.
#[derive(Debug, Clone)]
pub struct ProjectPaths {
    root: PathBuf,
}

impl ProjectPaths {
    /// Wrap an existing directory path.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Absolute project root.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Path to `project.json`.
    #[must_use]
    pub fn project_file(&self) -> PathBuf {
        self.root.join("project.json")
    }

    /// Directory holding imported media.
    #[must_use]
    pub fn media_dir(&self) -> PathBuf {
        self.root.join("media")
    }

    /// Directory holding peak caches.
    #[must_use]
    pub fn peaks_dir(&self) -> PathBuf {
        self.root.join("peaks")
    }

    /// Directory holding renders and exports.
    #[must_use]
    pub fn renders_dir(&self) -> PathBuf {
        self.root.join("renders")
    }

    /// Resolve a project-relative media path without allowing escapes.
    pub fn resolve_relative(&self, relative: &str) -> Result<PathBuf, CoreError> {
        let candidate = Path::new(relative);
        if candidate.is_absolute()
            || candidate
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            return Err(CoreError::validation(format!(
                "media path escapes the project: {relative}"
            )));
        }
        Ok(self.root.join(candidate))
    }

    /// Create the directory tree.
    pub fn ensure_layout(&self) -> Result<(), CoreError> {
        for directory in [
            self.root.clone(),
            self.media_dir(),
            self.peaks_dir(),
            self.renders_dir(),
        ] {
            fs::create_dir_all(&directory)?;
        }
        Ok(())
    }

    /// Create a project directory and write its `project.json`.
    pub fn create(&self, project: &Project) -> Result<(), CoreError> {
        self.ensure_layout()?;
        self.save(project)
    }

    /// Open a project directory (or a direct `project.json` path).
    pub fn open(path: impl AsRef<Path>) -> Result<(Self, Project), CoreError> {
        let path = path.as_ref();
        let root = if path.is_file() || path.ends_with("project.json") {
            path.parent()
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf()
        } else {
            path.to_path_buf()
        };
        let paths = Self::new(root);
        let project = paths.load()?;
        Ok((paths, project))
    }

    /// Load `project.json`.
    pub fn load(&self) -> Result<Project, CoreError> {
        let text = fs::read_to_string(self.project_file())?;
        schema::from_json(&text)
    }

    /// Atomically write `project.json`.
    pub fn save(&self, project: &Project) -> Result<(), CoreError> {
        self.ensure_layout()?;
        let text = schema::to_json(project)?;
        write_atomic(&self.project_file(), text.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_open_and_save() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("demo.alp");
        let paths = ProjectPaths::new(&root);
        let mut project = Project::new("demo").expect("project");
        paths.create(&project).expect("create");
        assert!(paths.media_dir().is_dir());
        assert!(paths.peaks_dir().is_dir());

        project.name = "renamed".into();
        paths.save(&project).expect("save");
        let (_, loaded) = ProjectPaths::open(&root).expect("open");
        assert_eq!(loaded.name, "renamed");
    }

    #[test]
    fn relative_paths_cannot_escape() {
        let paths = ProjectPaths::new("/tmp/project");
        assert!(paths.resolve_relative("media/a.wav").is_ok());
        assert!(paths.resolve_relative("../secret").is_err());
        assert!(paths.resolve_relative("/etc/passwd").is_err());
    }
}
