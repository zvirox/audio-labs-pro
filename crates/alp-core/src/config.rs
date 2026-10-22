//! XDG application directories.
//!
//! The app keeps configuration, durable data (models, voices) and caches apart,
//! following the XDG base-directory specification. Tests can redirect every
//! root with `ALP_CONFIG_HOME`, `ALP_DATA_HOME` and `ALP_CACHE_HOME`.

use std::path::PathBuf;

use crate::CoreError;

/// Resolved directories used by Audio Lab Pro.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppDirs {
    /// Configuration (theme, devices, preferences).
    pub config: PathBuf,
    /// Durable data (models, voice profiles).
    pub data: PathBuf,
    /// Rebuildable caches.
    pub cache: PathBuf,
}

impl AppDirs {
    /// Resolve directories from the environment and XDG defaults.
    pub fn discover() -> Result<Self, CoreError> {
        let config = env_root("ALP_CONFIG_HOME")
            .or_else(|| dirs::config_dir().map(|path| path.join("alp")))
            .ok_or(CoreError::MissingDirectory("config"))?;
        let data = env_root("ALP_DATA_HOME")
            .or_else(|| dirs::data_dir().map(|path| path.join("alp")))
            .ok_or(CoreError::MissingDirectory("data"))?;
        let cache = env_root("ALP_CACHE_HOME")
            .or_else(|| dirs::cache_dir().map(|path| path.join("alp")))
            .ok_or(CoreError::MissingDirectory("cache"))?;
        Ok(Self {
            config,
            data,
            cache,
        })
    }

    /// Build explicit roots (tests, portable mode).
    #[must_use]
    pub fn with_roots(
        config: impl Into<PathBuf>,
        data: impl Into<PathBuf>,
        cache: impl Into<PathBuf>,
    ) -> Self {
        Self {
            config: config.into(),
            data: data.into(),
            cache: cache.into(),
        }
    }

    /// Create all three roots.
    pub fn create(&self) -> Result<(), CoreError> {
        for directory in [&self.config, &self.data, &self.cache] {
            std::fs::create_dir_all(directory)?;
        }
        Ok(())
    }

    /// Directory for downloaded models.
    #[must_use]
    pub fn models_dir(&self) -> PathBuf {
        self.data.join("models")
    }

    /// Directory for the independent voice-profile library.
    #[must_use]
    pub fn voices_dir(&self) -> PathBuf {
        self.data.join("voices")
    }

    /// Default parent directory for projects.
    #[must_use]
    pub fn projects_dir(&self) -> PathBuf {
        self.data.join("projects")
    }

    /// Directory for crash-recovery autosaves.
    #[must_use]
    pub fn autosave_dir(&self) -> PathBuf {
        self.data.join("autosave")
    }
}

fn env_root(variable: &str) -> Option<PathBuf> {
    std::env::var_os(variable)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_roots_are_used_verbatim() {
        let dirs = AppDirs::with_roots("/tmp/c", "/tmp/d", "/tmp/k");
        assert_eq!(dirs.config, PathBuf::from("/tmp/c"));
        assert_eq!(dirs.models_dir(), PathBuf::from("/tmp/d/models"));
        assert_eq!(dirs.voices_dir(), PathBuf::from("/tmp/d/voices"));
    }
}
