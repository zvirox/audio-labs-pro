//! Filesystem plugin scanner discovering CLAP and LV2 bundles across standard Linux paths.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

use crate::clap::scan_clap_file;
use crate::error::PluginError;
use crate::lv2::scan_lv2_bundle;
use crate::types::PluginDescriptor;

/// Plugin scanner managing discovery paths and metadata caching.
#[derive(Debug, Clone)]
pub struct PluginScanner {
    clap_paths: Vec<PathBuf>,
    lv2_paths: Vec<PathBuf>,
    cache_path: Option<PathBuf>,
}

impl Default for PluginScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginScanner {
    /// Initialize a new scanner with standard Linux audio paths.
    #[must_use]
    pub fn new() -> Self {
        let mut clap_paths = vec![
            PathBuf::from("/usr/lib/clap"),
            PathBuf::from("/usr/local/lib/clap"),
        ];
        if let Ok(home) = env::var("HOME") {
            clap_paths.push(PathBuf::from(home).join(".clap"));
        }
        if let Ok(custom) = env::var("CLAP_PATH") {
            for p in custom.split(':') {
                if !p.is_empty() {
                    clap_paths.push(PathBuf::from(p));
                }
            }
        }

        let mut lv2_paths = vec![
            PathBuf::from("/usr/lib/lv2"),
            PathBuf::from("/usr/local/lib/lv2"),
        ];
        if let Ok(home) = env::var("HOME") {
            lv2_paths.push(PathBuf::from(home).join(".lv2"));
        }
        if let Ok(custom) = env::var("LV2_PATH") {
            for p in custom.split(':') {
                if !p.is_empty() {
                    lv2_paths.push(PathBuf::from(p));
                }
            }
        }

        let cache_path = if let Ok(cache_home) = env::var("XDG_CACHE_HOME") {
            Some(PathBuf::from(cache_home).join("alp").join("plugins.json"))
        } else if let Ok(home) = env::var("HOME") {
            Some(
                PathBuf::from(home)
                    .join(".cache")
                    .join("alp")
                    .join("plugins.json"),
            )
        } else {
            None
        };

        Self {
            clap_paths,
            lv2_paths,
            cache_path,
        }
    }

    /// Add a custom discovery search directory.
    pub fn add_search_path(&mut self, path: impl AsRef<Path>) {
        let p = path.as_ref().to_path_buf();
        self.clap_paths.push(p.clone());
        self.lv2_paths.push(p);
    }

    /// Set an explicit cache file path.
    pub fn set_cache_path(&mut self, path: impl AsRef<Path>) {
        self.cache_path = Some(path.as_ref().to_path_buf());
    }

    /// Scan all configured paths for CLAP and LV2 plugins.
    pub fn scan(&self) -> Vec<PluginDescriptor> {
        let mut descriptors = Vec::new();

        // 1. Scan CLAP paths
        for dir in &self.clap_paths {
            if dir.is_dir() {
                self.scan_clap_dir(dir, &mut descriptors);
            }
        }

        // 2. Scan LV2 paths
        for dir in &self.lv2_paths {
            if dir.is_dir() {
                self.scan_lv2_dir(dir, &mut descriptors);
            }
        }

        // Remove duplicate descriptors by ID
        descriptors.sort_by(|a, b| a.id.cmp(&b.id));
        descriptors.dedup_by(|a, b| a.id == b.id);

        if let Some(ref cache) = self.cache_path {
            let _ = self.save_cache(cache, &descriptors);
        }

        descriptors
    }

    fn scan_clap_dir(&self, dir: &Path, results: &mut Vec<PluginDescriptor>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file()
                    && p.extension()
                        .is_some_and(|ext| ext == "clap" || ext == "so")
                {
                    match scan_clap_file(&p) {
                        Ok(mut descs) => results.append(&mut descs),
                        Err(err) => warn!("failed scanning CLAP file {}: {err}", p.display()),
                    }
                }
            }
        }
    }

    fn scan_lv2_dir(&self, dir: &Path, results: &mut Vec<PluginDescriptor>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() && p.extension().is_some_and(|ext| ext == "lv2") {
                    match scan_lv2_bundle(&p) {
                        Ok(mut descs) => results.append(&mut descs),
                        Err(err) => warn!("failed scanning LV2 bundle {}: {err}", p.display()),
                    }
                }
            }
        }
    }

    /// Load cached descriptors from disk if present.
    pub fn load_cache(&self) -> Option<Vec<PluginDescriptor>> {
        let path = self.cache_path.as_ref()?;
        let data = fs::read_to_string(path).ok()?;
        serde_json::from_str(&data).ok()
    }

    fn save_cache(&self, path: &Path, descriptors: &[PluginDescriptor]) -> Result<(), PluginError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(descriptors).map_err(|e| PluginError::State {
            plugin_id: "scanner".into(),
            message: e.to_string(),
        })?;
        fs::write(path, json)?;
        info!(
            "cached {} plugin descriptors to {}",
            descriptors.len(),
            path.display()
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn scanner_respects_custom_paths_and_caching() {
        let dir = tempdir().expect("tempdir");
        let cache_file = dir.path().join("plugins_test.json");

        let mut scanner = PluginScanner::new();
        scanner.set_cache_path(&cache_file);
        scanner.add_search_path(dir.path());

        let plugins = scanner.scan();
        assert!(cache_file.exists());

        let loaded = scanner.load_cache().expect("load cache");
        assert_eq!(plugins, loaded);
    }
}
