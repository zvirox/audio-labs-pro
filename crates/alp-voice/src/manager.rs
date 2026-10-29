//! Model manager handling catalogue storage, verification, licenses and disk space.

use alp_core::CoreError;
use std::fs;
use std::path::{Path, PathBuf};

use crate::model::{ModelId, ModelManifest, ModelStatus, standard_catalogue};
use crate::sha256;

/// Configuration for the model manager.
#[derive(Debug, Clone)]
pub struct ModelManagerConfig {
    /// Directory where downloaded models are stored.
    pub storage_dir: PathBuf,
    /// Whether the model manager is operating in offline mode.
    pub offline_mode: bool,
}

impl Default for ModelManagerConfig {
    fn default() -> Self {
        let base_dir = dirs::data_dir()
            .map(|p| p.join("alp").join("models"))
            .unwrap_or_else(|| PathBuf::from("./.local_models"));
        Self {
            storage_dir: base_dir,
            offline_mode: false,
        }
    }
}

/// Central manager for AI models, licences, and on-disk assets.
#[derive(Debug)]
pub struct ModelManager {
    config: ModelManagerConfig,
    catalogue: Vec<ModelManifest>,
}

impl ModelManager {
    /// Create a model manager with the standard catalogue and default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(ModelManagerConfig::default())
    }

    /// Create a model manager with specific configuration.
    ///
    /// License acceptance markers previously written under
    /// `<storage_dir>/licenses/` are restored into the catalogue, so gates set
    /// by one process (`alp-cli model accept-license`) stay open for the next
    /// (daemon, UI, retries). Read failures (e.g. a missing directory) are
    /// silently treated as "nothing accepted yet".
    #[must_use]
    pub fn with_config(config: ModelManagerConfig) -> Self {
        let mut manager = Self {
            config,
            catalogue: standard_catalogue(),
        };
        manager.load_license_markers();
        manager
    }

    /// Path to the directory holding per-model license acceptance markers.
    fn licenses_dir(&self) -> PathBuf {
        self.config.storage_dir.join("licenses")
    }

    /// Restore persisted `<model-id>.accepted` markers into the catalogue.
    fn load_license_markers(&mut self) {
        let Ok(entries) = fs::read_dir(self.licenses_dir()) else {
            return;
        };
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let Some(name) = file_name.to_str() else {
                continue;
            };
            let Some(model_id) = name.strip_suffix(".accepted") else {
                continue;
            };
            if let Some(manifest) = self
                .catalogue
                .iter_mut()
                .find(|m| m.id.as_str() == model_id)
            {
                manifest.license.accepted = true;
            }
        }
    }

    /// Path to the root model storage directory.
    #[must_use]
    pub fn storage_dir(&self) -> &Path {
        &self.config.storage_dir
    }

    /// Path to an individual model directory on disk.
    #[must_use]
    pub fn model_dir(&self, id: &ModelId) -> PathBuf {
        self.config.storage_dir.join(id.as_str())
    }

    /// Whether offline mode is enabled.
    #[must_use]
    pub fn is_offline(&self) -> bool {
        self.config.offline_mode
    }

    /// Set offline mode.
    pub fn set_offline(&mut self, offline: bool) {
        self.config.offline_mode = offline;
    }

    /// Return all model manifests in the catalogue.
    #[must_use]
    pub fn catalogue(&self) -> &[ModelManifest] {
        &self.catalogue
    }

    /// Find a model manifest by identifier.
    #[must_use]
    pub fn get_manifest(&self, id: &ModelId) -> Option<&ModelManifest> {
        self.catalogue.iter().find(|m| &m.id == id)
    }

    /// Accept the license terms for a model.
    pub fn accept_license(&mut self, id: &ModelId) -> Result<(), CoreError> {
        let manifest = self
            .catalogue
            .iter_mut()
            .find(|m| &m.id == id)
            .ok_or_else(|| CoreError::validation(format!("model {id} not found")))?;
        manifest.license.accepted = true;
        // Persist the acceptance so later processes (CLI reruns, the daemon,
        // the UI) keep the gate open without re-prompting.
        fs::create_dir_all(self.licenses_dir())?;
        fs::write(
            self.licenses_dir().join(format!("{}.accepted", id.0)),
            b"accepted\n",
        )?;
        Ok(())
    }

    /// Current status of a model on disk.
    #[must_use]
    pub fn status(&self, id: &ModelId) -> ModelStatus {
        let Some(manifest) = self.get_manifest(id) else {
            return ModelStatus::NotInstalled;
        };

        if manifest.is_builtin {
            return ModelStatus::Verified {
                path: PathBuf::from("builtin"),
            };
        }

        let dir = self.model_dir(id);
        if !dir.is_dir() {
            return ModelStatus::NotInstalled;
        }

        if manifest.is_installed_at(&dir) {
            ModelStatus::Installed { path: dir }
        } else {
            ModelStatus::NotInstalled
        }
    }

    /// Verify SHA-256 checksums of all files for an installed model.
    pub fn verify(&self, id: &ModelId) -> Result<ModelStatus, CoreError> {
        let manifest = self
            .get_manifest(id)
            .ok_or_else(|| CoreError::validation(format!("model {id} not found")))?;

        if manifest.is_builtin {
            return Ok(ModelStatus::Verified {
                path: PathBuf::from("builtin"),
            });
        }

        let dir = self.model_dir(id);
        if !dir.is_dir() {
            return Ok(ModelStatus::NotInstalled);
        }

        for file in &manifest.files {
            let file_path = dir.join(&file.filename);
            let bytes = fs::read(&file_path)?;

            if bytes.len() as u64 != file.size_bytes {
                return Ok(ModelStatus::Corrupt {
                    reason: format!(
                        "file {} length mismatch (expected {}, got {})",
                        file.filename,
                        file.size_bytes,
                        bytes.len()
                    ),
                });
            }

            let hex = sha256::digest_hex(&bytes);
            if !hex.eq_ignore_ascii_case(&file.sha256) {
                return Ok(ModelStatus::Corrupt {
                    reason: format!(
                        "checksum mismatch on {}: expected {}, got {}",
                        file.filename, file.sha256, hex
                    ),
                });
            }
        }

        Ok(ModelStatus::Verified { path: dir })
    }

    /// Install model files directly from memory or local buffers into the model directory.
    pub fn install_file(
        &self,
        id: &ModelId,
        filename: &str,
        data: &[u8],
    ) -> Result<PathBuf, CoreError> {
        let dir = self.model_dir(id);
        fs::create_dir_all(&dir)?;

        let target = dir.join(filename);
        let part_target = dir.join(format!("{filename}.part"));

        // Resumable write pattern: write to .part then rename atomically
        fs::write(&part_target, data)?;
        fs::rename(&part_target, &target)?;

        Ok(target)
    }

    /// Remove a model from disk.
    pub fn remove(&self, id: &ModelId) -> Result<(), CoreError> {
        let dir = self.model_dir(id);
        if dir.exists() {
            fs::remove_dir_all(&dir)?;
        }
        Ok(())
    }

    /// Register an extra manifest (test helper for custom catalogues).
    #[cfg(test)]
    pub(crate) fn register_manifest(&mut self, manifest: ModelManifest) {
        self.catalogue.push(manifest);
    }
}

impl Default for ModelManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn builtin_model_is_always_verified() {
        let mgr = ModelManager::new();
        let builtin_id = ModelId::new("builtin-acoustic");
        assert!(mgr.status(&builtin_id).is_ready());
    }

    #[test]
    fn license_acceptance_persists_across_managers() {
        let temp = tempdir().expect("tempdir");
        let id = ModelId::new("kokoro-82m");

        let mut first = ModelManager::with_config(ModelManagerConfig {
            storage_dir: temp.path().to_path_buf(),
            offline_mode: false,
        });
        assert!(!first.get_manifest(&id).expect("manifest").license.accepted);
        first.accept_license(&id).expect("accept");

        // A fresh instance (e.g. the next CLI invocation) keeps the gate open.
        let second = ModelManager::with_config(ModelManagerConfig {
            storage_dir: temp.path().to_path_buf(),
            offline_mode: false,
        });
        assert!(second.get_manifest(&id).expect("manifest").license.accepted);
        assert!(
            !second
                .get_manifest(&ModelId::new("moonshine-tiny"))
                .expect("manifest")
                .license
                .accepted
        );
    }

    #[test]
    fn file_installation_and_verification_round_trip() {
        let temp = tempdir().expect("tempdir");
        let config = ModelManagerConfig {
            storage_dir: temp.path().to_path_buf(),
            offline_mode: false,
        };
        let mut mgr = ModelManager::with_config(config);

        let test_id = ModelId::new("test-model");
        let payload = b"Hello, Audio Labs Pro test payload.";
        let payload_sha = sha256::digest_hex(payload);

        let manifest = ModelManifest {
            id: test_id.clone(),
            name: "Test Model".into(),
            description: "Test".into(),
            kind: crate::model::ModelKind::TtsLight,
            author: "Test".into(),
            version: "1.0.0".into(),
            license: crate::model::ModelLicense {
                name: "MIT".into(),
                spdx: "MIT".into(),
                terms: "MIT terms".into(),
                accepted: true,
            },
            files: vec![crate::model::ModelFile {
                filename: "weights.bin".into(),
                size_bytes: payload.len() as u64,
                sha256: payload_sha,
                url: "local://weights.bin".into(),
            }],
            is_builtin: false,
            voice_names: vec!["Test Voice".into()],
        };

        mgr.catalogue.push(manifest);

        assert_eq!(mgr.status(&test_id), ModelStatus::NotInstalled);

        // Install file
        mgr.install_file(&test_id, "weights.bin", payload)
            .expect("install");

        assert!(mgr.status(&test_id).is_ready());

        let verified = mgr.verify(&test_id).expect("verify");
        assert!(matches!(verified, ModelStatus::Verified { .. }));

        // Remove
        mgr.remove(&test_id).expect("remove");
        assert_eq!(mgr.status(&test_id), ModelStatus::NotInstalled);
    }
}
