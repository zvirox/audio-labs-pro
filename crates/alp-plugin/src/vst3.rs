//! VST3 bundle discovery.
//!
//! This build discovers `.vst3` bundles and records their shared-library
//! payloads as descriptors. It deliberately does **not** claim to host the
//! VST3 C++ COM ABI: instantiating a VST3 descriptor returns a clear error in
//! `ActivePlugin::instantiate`. Discovery is still useful for inventory and
//! for the plugin browser.

use std::path::{Path, PathBuf};

use crate::error::PluginError;
use crate::types::{PluginCategory, PluginDescriptor, PluginFormat};

/// Maximum recursion depth inside a VST3 bundle.
const MAX_BUNDLE_DEPTH: usize = 4;

/// Scan a `.vst3` bundle directory and return a descriptor for its binary.
pub fn scan_vst3_bundle(path: &Path) -> Result<PluginDescriptor, PluginError> {
    if !path.is_dir() {
        return Err(PluginError::Discovery {
            path: path.to_path_buf(),
            message: "VST3 bundle is not a directory".into(),
        });
    }
    let binary = find_shared_library(path, 0).ok_or_else(|| PluginError::Discovery {
        path: path.to_path_buf(),
        message: "no shared library found under Contents/<arch>".into(),
    })?;
    let name = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "VST3 Plugin".to_owned());
    Ok(PluginDescriptor {
        id: format!("vst3:{name}"),
        name,
        vendor: "VST3".to_owned(),
        version: "unknown".to_owned(),
        format: PluginFormat::Vst3,
        category: PluginCategory::Effect,
        path: binary,
        audio_inputs: 2,
        audio_outputs: 2,
        param_count: 0,
    })
}

fn find_shared_library(dir: &Path, depth: usize) -> Option<PathBuf> {
    if depth > MAX_BUNDLE_DEPTH {
        return None;
    }
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect();
    entries.sort();
    for entry in entries {
        if entry.is_file()
            && entry
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("so"))
        {
            return Some(entry);
        }
    }
    for entry in std::fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
    {
        if entry.is_dir()
            && let Some(found) = find_shared_library(&entry, depth + 1)
        {
            return Some(found);
        }
    }
    None
}

/// Error returned when a VST3 descriptor is instantiated.
#[must_use]
pub fn instantiation_error(desc: &PluginDescriptor) -> PluginError {
    PluginError::Instantiation {
        plugin_id: desc.id.clone(),
        message: "the VST3 C++ COM ABI host is not implemented in this build; the bundle was discovered but cannot be loaded".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_directory_is_rejected() {
        let error = scan_vst3_bundle(Path::new("/definitely/not/a.bundle")).expect_err("error");
        assert!(error.to_string().contains("not a directory"));
    }

    #[test]
    fn discovers_bundle_binary() {
        let dir = tempfile::tempdir().expect("tempdir");
        let bundle = dir.path().join("Test.vst3");
        let binary_dir = bundle.join("Contents/x86_64-linux");
        std::fs::create_dir_all(&binary_dir).expect("bundle dirs");
        let binary = binary_dir.join("Test.so");
        std::fs::write(&binary, b"not a real library").expect("binary");
        let descriptor = scan_vst3_bundle(&bundle).expect("descriptor");
        assert_eq!(descriptor.format, PluginFormat::Vst3);
        assert_eq!(descriptor.id, "vst3:Test");
        assert_eq!(descriptor.path, binary);
        assert!(
            instantiation_error(&descriptor)
                .to_string()
                .contains("not implemented")
        );
    }
}
