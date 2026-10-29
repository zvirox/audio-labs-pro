//! Voice profile management, library storage, prebuilt voices, and `.alpvoice` packaging.

use alp_core::{AppDirs, CoreError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::VoiceKind;
use crate::zip::{ZipReader, ZipWriter};

/// Validate an untrusted profile identifier before it is ever used in a path.
///
/// Profile ids arrive from imported bundles, so they are attacker-controlled:
/// reject empty ids, path separators, and any non-`Normal` component
/// (`..`, `.`, absolute prefixes) so `root.join(id)` can never escape the store.
fn validate_profile_id(id: &str) -> Result<(), CoreError> {
    let invalid = id.trim().is_empty()
        || id.contains('/')
        || id.contains('\\')
        || Path::new(id)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)));
    if invalid {
        return Err(CoreError::validation(format!(
            "invalid voice profile id {id:?}: ids must be plain names without path separators or traversal components"
        )));
    }
    Ok(())
}

/// Parameters customizing speech generation for a voice profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceParams {
    /// Speed multiplier (`1.0` is normal speed).
    pub speed: f32,
    /// Pitch adjustment in semitones (`0.0` is neutral).
    pub pitch: f32,
    /// Energy / vocal effort (`1.0` is neutral).
    pub energy: f32,
    /// Expressive emotion / style tag ("neutral", "warm", "excited", "whisper").
    pub emotion: String,
}

impl Default for VoiceParams {
    fn default() -> Self {
        Self {
            speed: 1.0,
            pitch: 0.0,
            energy: 1.0,
            emotion: "neutral".into(),
        }
    }
}

/// Human legal consent attestation record required for voice cloning.
///
/// Under build-plan safety rules, this gate is human-only: automated agents,
/// scripts, and APIs are prohibited from bypassing or synthesizing consent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsentRecord {
    /// Name of the individual whose voice is being cloned.
    pub speaker_name: String,
    /// Identity of the user attesting to having obtained explicit permission.
    pub attested_by: String,
    /// Unix timestamp in seconds when the consent was granted.
    pub timestamp: u64,
    /// Full text of the legal consent statement acknowledged by the user.
    pub statement: String,
}

impl ConsentRecord {
    /// Create and validate a human consent record.
    pub fn new(
        speaker_name: impl Into<String>,
        attested_by: impl Into<String>,
        statement: impl Into<String>,
    ) -> Result<Self, CoreError> {
        let speaker = speaker_name.into();
        let attester = attested_by.into();
        let stmt = statement.into();

        if speaker.trim().is_empty() {
            return Err(CoreError::validation("speaker name cannot be empty"));
        }
        if attester.trim().is_empty() {
            return Err(CoreError::validation("attesting user cannot be empty"));
        }
        if stmt.trim().len() < 10 {
            return Err(CoreError::validation("consent statement is too short"));
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Ok(Self {
            speaker_name: speaker,
            attested_by: attester,
            timestamp,
            statement: stmt,
        })
    }
}

/// Validate a consent record obtained from an untrusted source (an imported
/// bundle) against the same rules [`ConsentRecord::new`] enforces when a human
/// creates one: the attestation must identify a speaker, an attester, and a
/// meaningful statement.
fn validate_consent_record(consent: &ConsentRecord) -> Result<(), CoreError> {
    if consent.speaker_name.trim().is_empty() {
        return Err(CoreError::validation(
            "invalid consent record: speaker name cannot be empty",
        ));
    }
    if consent.attested_by.trim().is_empty() {
        return Err(CoreError::validation(
            "invalid consent record: attesting user cannot be empty",
        ));
    }
    if consent.statement.trim().len() < 10 {
        return Err(CoreError::validation(
            "invalid consent record: consent statement is too short",
        ));
    }
    Ok(())
}

/// A reusable voice profile independent of any individual project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceProfile {
    /// Stable profile identifier.
    pub id: String,
    /// Friendly display name.
    pub name: String,
    /// Whether the voice is built-in, cloned, or designed.
    pub kind: VoiceKind,
    /// Backend engine that created and executes this voice.
    pub backend: String,
    /// Optional relative path to stored reference audio.
    pub reference_audio: Option<PathBuf>,
    /// Optional reference transcript for continuation synthesis.
    pub transcript: Option<String>,
    /// Optional cached neural speaker embedding.
    pub cached_embedding: Option<Vec<f32>>,
    /// Descriptive tags (e.g. `["clean", "warm", "narration"]`).
    pub tags: Vec<String>,
    /// Default generation parameters.
    pub default_params: VoiceParams,
    /// Unix creation timestamp.
    pub created_at: u64,
    /// Mandatory consent record for cloned voices.
    pub consent: Option<ConsentRecord>,
}

impl VoiceProfile {
    /// Create a new built-in voice profile.
    #[must_use]
    pub fn builtin(id: impl Into<String>, name: impl Into<String>, tags: Vec<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind: VoiceKind::Builtin,
            backend: "builtin-acoustic".into(),
            reference_audio: None,
            transcript: None,
            cached_embedding: None,
            tags,
            default_params: VoiceParams::default(),
            created_at: 0,
            consent: None,
        }
    }
}

/// Persistent storage manager for voice profiles on disk.
#[derive(Debug, Clone)]
pub struct ProfileStore {
    root_dir: PathBuf,
}

impl ProfileStore {
    /// Open the default profile store located in the user's data directory.
    pub fn default_store() -> Result<Self, CoreError> {
        let app_dirs = AppDirs::discover().map_err(|e| CoreError::validation(e.to_string()))?;
        let voices_dir = app_dirs.voices_dir();
        Self::new(voices_dir)
    }

    /// Open or create a profile store at an explicit root directory.
    pub fn new(root_dir: impl Into<PathBuf>) -> Result<Self, CoreError> {
        let root = root_dir.into();
        fs::create_dir_all(&root).map_err(CoreError::Io)?;

        let store = Self { root_dir: root };
        store.ensure_prebuilt_profiles()?;
        Ok(store)
    }

    /// Root directory of the profile store.
    #[must_use]
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    /// Ensure prebuilt system profiles exist.
    fn ensure_prebuilt_profiles(&self) -> Result<(), CoreError> {
        let defaults = [
            (
                "aura",
                "Aura",
                vec!["clean".into(), "neutral".into(), "english".into()],
            ),
            (
                "atlas",
                "Atlas",
                vec!["warm".into(), "narration".into(), "english".into()],
            ),
            (
                "echo",
                "Echo",
                vec!["clear".into(), "crisp".into(), "english".into()],
            ),
            (
                "sol",
                "Sol",
                vec!["bright".into(), "expressive".into(), "english".into()],
            ),
        ];

        for (id, name, tags) in defaults {
            if self.get_profile(id).is_none() {
                let profile = VoiceProfile::builtin(id, name, tags);
                self.save_profile(&profile)?;
            }
        }
        Ok(())
    }

    /// List all voice profiles in the library.
    #[must_use]
    pub fn list_profiles(&self) -> Vec<VoiceProfile> {
        let mut profiles = Vec::new();
        let Ok(entries) = fs::read_dir(&self.root_dir) else {
            return profiles;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let json_path = path.join("profile.json");
                if let Ok(content) = fs::read_to_string(&json_path)
                    && let Ok(profile) = serde_json::from_str::<VoiceProfile>(&content)
                {
                    profiles.push(profile);
                }
            }
        }

        profiles.sort_by(|a, b| a.name.cmp(&b.name));
        profiles
    }

    /// Get a profile by its ID.
    #[must_use]
    pub fn get_profile(&self, id: &str) -> Option<VoiceProfile> {
        if validate_profile_id(id).is_err() {
            return None;
        }
        let profile_dir = self.root_dir.join(id);
        let json_path = profile_dir.join("profile.json");
        let content = fs::read_to_string(json_path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Save or update a voice profile in the store.
    pub fn save_profile(&self, profile: &VoiceProfile) -> Result<(), CoreError> {
        validate_profile_id(&profile.id)?;

        // Enforce consent for cloned profiles
        if profile.kind == VoiceKind::Cloned && profile.consent.is_none() {
            return Err(CoreError::validation(
                "cannot save a cloned voice profile without a valid human consent record",
            ));
        }

        // A consent record only makes sense on a cloned profile.
        if profile.kind != VoiceKind::Cloned && profile.consent.is_some() {
            return Err(CoreError::validation(
                "cannot save a non-cloned profile carrying a consent record",
            ));
        }
        if let Some(consent) = &profile.consent {
            validate_consent_record(consent)?;
        }

        let profile_dir = self.root_dir.join(&profile.id);
        fs::create_dir_all(&profile_dir).map_err(CoreError::Io)?;

        let json_path = profile_dir.join("profile.json");
        let content = serde_json::to_string_pretty(profile)?;
        fs::write(json_path, content).map_err(CoreError::Io)?;
        Ok(())
    }

    /// Delete a voice profile by its ID.
    pub fn delete_profile(&self, id: &str) -> Result<bool, CoreError> {
        validate_profile_id(id)?;
        let profile_dir = self.root_dir.join(id);
        if profile_dir.exists() {
            fs::remove_dir_all(profile_dir).map_err(CoreError::Io)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Export a voice profile into a standalone `.alpvoice` ZIP bundle.
    pub fn export_bundle(&self, id: &str, output_path: &Path) -> Result<(), CoreError> {
        let profile = self
            .get_profile(id)
            .ok_or_else(|| CoreError::validation(format!("profile '{id}' not found")))?;

        let profile_dir = self.root_dir.join(id);
        let profile_json = serde_json::to_vec_pretty(&profile)?;

        let file = fs::File::create(output_path).map_err(CoreError::Io)?;
        let mut zip = ZipWriter::new(file);

        zip.add_file("profile.json", &profile_json)?;

        // Attach reference audio if present
        if let Some(ref_rel) = &profile.reference_audio {
            let full_ref_path = profile_dir.join(ref_rel);
            // A crafted profile.json can point anywhere; never export files
            // from outside this profile's own directory.
            if full_ref_path.exists() {
                let profile_root = profile_dir.canonicalize().map_err(CoreError::Io)?;
                let resolved_ref = full_ref_path.canonicalize().map_err(CoreError::Io)?;
                if !resolved_ref.starts_with(&profile_root) {
                    return Err(CoreError::validation(format!(
                        "reference audio {ref_rel:?} points outside the profile directory"
                    )));
                }
                let ref_bytes = fs::read(resolved_ref).map_err(CoreError::Io)?;
                zip.add_file("reference.wav", &ref_bytes)?;
            }
        }

        // Attach cached embedding if present
        if let Some(emb) = &profile.cached_embedding {
            let mut emb_bytes = Vec::with_capacity(emb.len() * 4);
            for &f in emb {
                emb_bytes.extend_from_slice(&f.to_le_bytes());
            }
            zip.add_file("embedding.bin", &emb_bytes)?;
        }

        zip.finish()?;
        Ok(())
    }

    /// Import a voice profile from a `.alpvoice` ZIP bundle.
    pub fn import_bundle(&self, bundle_path: &Path) -> Result<VoiceProfile, CoreError> {
        let bytes = fs::read(bundle_path).map_err(CoreError::Io)?;
        let reader = ZipReader::new(&bytes)?;

        let json_bytes = reader
            .get("profile.json")
            .ok_or_else(|| CoreError::validation("missing profile.json in .alpvoice package"))?;
        let mut profile: VoiceProfile = serde_json::from_slice(json_bytes)?;

        validate_profile_id(&profile.id)?;

        // Safety check: cloned profiles must carry a consent record, and an
        // imported record must satisfy the same validations the app applies
        // when a human creates consent. A bundle alone is not authority; it
        // must at least be structurally truthful before it is stored.
        if profile.kind == VoiceKind::Cloned && profile.consent.is_none() {
            return Err(CoreError::validation(
                "import rejected: cloned profile in package is missing required consent record",
            ));
        }
        if profile.kind != VoiceKind::Cloned && profile.consent.is_some() {
            return Err(CoreError::validation(
                "import rejected: consent record attached to a non-cloned profile",
            ));
        }
        if let Some(consent) = &profile.consent {
            validate_consent_record(consent)?;
        }

        let profile_dir = self.root_dir.join(&profile.id);
        fs::create_dir_all(&profile_dir).map_err(CoreError::Io)?;

        if let Some(ref_data) = reader.get("reference.wav") {
            let ref_path = profile_dir.join("reference.wav");
            fs::write(&ref_path, ref_data).map_err(CoreError::Io)?;
            profile.reference_audio = Some(PathBuf::from("reference.wav"));
        } else {
            // A bundle-declared reference path is untrusted: never preserve it
            // when the bundle did not actually ship the audio it points at.
            profile.reference_audio = None;
        }

        if let Some(emb_data) = reader.get("embedding.bin") {
            let mut emb = Vec::new();
            for chunk in emb_data.as_chunks::<4>().0 {
                let val = f32::from_le_bytes(*chunk);
                emb.push(val);
            }
            profile.cached_embedding = Some(emb);
        }

        self.save_profile(&profile)?;
        Ok(profile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn prebuilt_profiles_are_seeded() {
        let dir = tempdir().expect("tempdir");
        let store = ProfileStore::new(dir.path()).expect("store");
        let profiles = store.list_profiles();

        assert_eq!(profiles.len(), 4);
        assert!(store.get_profile("aura").is_some());
        assert!(store.get_profile("atlas").is_some());
    }

    #[test]
    fn cloned_profile_requires_consent() {
        let dir = tempdir().expect("tempdir");
        let store = ProfileStore::new(dir.path()).expect("store");

        let unconsented = VoiceProfile {
            id: "unauthorized".into(),
            name: "Unauthorized Clone".into(),
            kind: VoiceKind::Cloned,
            backend: "builtin-acoustic".into(),
            reference_audio: None,
            transcript: None,
            cached_embedding: None,
            tags: vec![],
            default_params: VoiceParams::default(),
            created_at: 100,
            consent: None,
        };

        // Attempting to save a cloned profile without consent must fail
        assert!(store.save_profile(&unconsented).is_err());

        // Adding valid consent succeeds
        let consent = ConsentRecord::new(
            "Jane Doe",
            "Studio Engineer",
            "I have explicit written authorization to create this voice model.",
        )
        .expect("consent");

        let mut consented = unconsented;
        consented.consent = Some(consent);
        assert!(store.save_profile(&consented).is_ok());
    }

    #[test]
    fn bundle_export_and_import_round_trip() {
        let dir = tempdir().expect("tempdir");
        let store = ProfileStore::new(dir.path()).expect("store");

        let bundle_path = dir.path().join("aura.alpvoice");
        store
            .export_bundle("aura", &bundle_path)
            .expect("export bundle");
        assert!(bundle_path.exists());

        // Import into second isolated store
        let dir2 = tempdir().expect("tempdir2");
        let store2 = ProfileStore::new(dir2.path()).expect("store2");
        let imported = store2.import_bundle(&bundle_path).expect("import bundle");

        assert_eq!(imported.id, "aura");
        assert_eq!(imported.name, "Aura");
    }

    fn write_bundle(
        dir: &Path,
        name: &str,
        profile: &VoiceProfile,
        reference: Option<&[u8]>,
    ) -> PathBuf {
        let path = dir.join(name);
        let file = fs::File::create(&path).expect("bundle file");
        let mut zip = ZipWriter::new(file);
        let json = serde_json::to_vec(profile).expect("serialize profile");
        zip.add_file("profile.json", &json)
            .expect("add profile.json");
        if let Some(ref_data) = reference {
            zip.add_file("reference.wav", ref_data)
                .expect("add reference");
        }
        zip.finish().expect("finish zip");
        path
    }

    #[test]
    fn import_rejects_traversal_profile_id() {
        let dir = tempdir().expect("tempdir");
        let store = ProfileStore::new(dir.path().join("store")).expect("store");
        let payload_dir = tempdir().expect("payloads");

        for id in [
            "../escape",
            "/etc/pwned",
            "nested/path",
            "back\\slash",
            "..",
        ] {
            let mut profile = VoiceProfile::builtin(id, "Evil", vec![]);
            profile.kind = VoiceKind::Designed;
            let bundle = write_bundle(payload_dir.path(), "evil.alpvoice", &profile, None);
            assert!(
                store.import_bundle(&bundle).is_err(),
                "profile id {id:?} must be rejected on import"
            );
        }

        // Nothing may have been created outside the store root.
        assert!(!dir.path().join("escape").exists());
        assert!(!dir.path().join("nested").exists());
        assert!(!dir.path().join("back").exists());
    }

    #[test]
    fn delete_profile_rejects_traversal_id() {
        let dir = tempdir().expect("tempdir");
        let store = ProfileStore::new(dir.path().join("store")).expect("store");

        let victim = dir.path().join("victim");
        fs::create_dir_all(&victim).expect("victim dir");
        fs::write(victim.join("keep.txt"), b"precious").expect("victim file");

        assert!(
            store.delete_profile("../victim").is_err(),
            "a traversal id must not delete outside the store"
        );
        assert!(victim.exists(), "victim directory must survive");
    }

    #[test]
    fn export_bundle_rejects_reference_audio_outside_profile_dir() {
        let dir = tempdir().expect("tempdir");
        let store = ProfileStore::new(dir.path().join("store")).expect("store");

        let secret = dir.path().join("secret.wav");
        fs::write(&secret, b"do not share").expect("secret file");

        let mut profile = VoiceProfile::builtin("leaky", "Leaky", vec![]);
        profile.reference_audio = Some(PathBuf::from("../../secret.wav"));
        store.save_profile(&profile).expect("save leaky profile");

        let bundle_path = dir.path().join("leaky.alpvoice");
        assert!(
            store.export_bundle("leaky", &bundle_path).is_err(),
            "reference_audio escaping the profile dir must be refused"
        );
        assert!(secret.exists(), "secret file must be untouched");
    }

    #[test]
    fn import_enforces_consent_content_validation() {
        let dir = tempdir().expect("tempdir");
        let store = ProfileStore::new(dir.path().join("store")).expect("store");
        let payload_dir = tempdir().expect("payloads");

        // Cloned profile whose consent record is structurally invalid.
        let mut forged = VoiceProfile::builtin("forged-clone", "Forged", vec![]);
        forged.kind = VoiceKind::Cloned;
        forged.consent = Some(ConsentRecord {
            speaker_name: String::new(),
            attested_by: "x".into(),
            timestamp: 1,
            statement: "short".into(),
        });
        let bundle = write_bundle(payload_dir.path(), "forged.alpvoice", &forged, None);
        assert!(
            store.import_bundle(&bundle).is_err(),
            "a consent record with empty speaker / short statement must be rejected"
        );
        assert!(store.get_profile("forged-clone").is_none());

        // A non-cloned profile carrying a consent record is inconsistent.
        let mut stray = VoiceProfile::builtin("stray-consent", "Stray", vec![]);
        stray.kind = VoiceKind::Designed;
        stray.consent = Some(ConsentRecord {
            speaker_name: "Someone".into(),
            attested_by: "me".into(),
            timestamp: 2,
            statement: "a perfectly long consent statement".into(),
        });
        let bundle = write_bundle(payload_dir.path(), "stray.alpvoice", &stray, None);
        assert!(
            store.import_bundle(&bundle).is_err(),
            "consent attached to a non-cloned profile must be rejected"
        );
    }
}
