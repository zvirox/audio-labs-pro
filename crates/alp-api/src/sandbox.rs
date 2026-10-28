//! Workspace filesystem sandboxing and path traversal guards.

use crate::error::ApiError;
use std::path::{Path, PathBuf};

/// Workspace path sandbox restricting file operations to allowed roots.
#[derive(Debug, Clone)]
pub struct WorkspaceSandbox {
    allowed_roots: Vec<PathBuf>,
}

impl WorkspaceSandbox {
    /// Create a sandbox initialized with one or more allowed root paths.
    pub fn new(roots: impl IntoIterator<Item = impl AsRef<Path>>) -> Result<Self, ApiError> {
        let mut canonical_roots = Vec::new();
        for r in roots {
            let path = r.as_ref();
            if path.exists() {
                canonical_roots.push(path.canonicalize()?);
            } else {
                canonical_roots.push(path.to_path_buf());
            }
        }
        if canonical_roots.is_empty() {
            return Err(ApiError::Validation(
                "sandbox requires at least one allowed root directory".into(),
            ));
        }
        Ok(Self {
            allowed_roots: canonical_roots,
        })
    }

    /// Add an additional directory root to the allowlist.
    pub fn add_root(&mut self, path: impl AsRef<Path>) -> Result<(), ApiError> {
        let path = path.as_ref();
        let canonical = if path.exists() {
            path.canonicalize()?
        } else {
            path.to_path_buf()
        };
        if !self.allowed_roots.contains(&canonical) {
            self.allowed_roots.push(canonical);
        }
        Ok(())
    }

    /// Validate that a requested target path resolves strictly inside one of the allowed roots.
    pub fn validate_path(&self, candidate: impl AsRef<Path>) -> Result<PathBuf, ApiError> {
        let candidate = candidate.as_ref();

        // Reject explicit parent directory components
        if candidate
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(ApiError::SandboxEscape(format!(
                "path traversal attempt with '..' detected: {}",
                candidate.display()
            )));
        }

        // Canonicalize candidate if it exists
        let resolved = if candidate.exists() {
            candidate.canonicalize()?
        } else if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            // Relative candidate against first root
            self.allowed_roots[0].join(candidate)
        };

        let inside = self
            .allowed_roots
            .iter()
            .any(|root| resolved.starts_with(root));
        if !inside {
            return Err(ApiError::SandboxEscape(format!(
                "path '{}' escapes allowed workspace sandbox directories",
                candidate.display()
            )));
        }

        Ok(resolved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn sandbox_allows_contained_paths() {
        let dir = tempdir().expect("tempdir");
        let sandbox = WorkspaceSandbox::new([dir.path()]).expect("sandbox");

        let valid_file = dir.path().join("audio.wav");
        std::fs::write(&valid_file, b"test").expect("write");

        let validated = sandbox.validate_path(&valid_file).expect("validate");
        assert_eq!(validated, valid_file.canonicalize().unwrap());
    }

    #[test]
    fn sandbox_blocks_parent_traversal() {
        let dir = tempdir().expect("tempdir");
        let sandbox = WorkspaceSandbox::new([dir.path()]).expect("sandbox");

        let bad_path = dir.path().join("../escape.wav");
        let result = sandbox.validate_path(&bad_path);
        assert!(result.is_err());
    }

    #[test]
    fn sandbox_blocks_unauthorized_absolute_paths() {
        let dir = tempdir().expect("tempdir");
        let sandbox = WorkspaceSandbox::new([dir.path()]).expect("sandbox");

        let outside = Path::new("/etc/passwd");
        let result = sandbox.validate_path(outside);
        assert!(result.is_err());
    }
}
