//! Model catalogue, manifests, licenses and installation status.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Unique identifier for an AI model.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ModelId(pub String);

impl ModelId {
    /// Create a new model identifier.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// String slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ModelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Category / purpose of a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelKind {
    /// Lightweight built-in or fast CPU TTS.
    TtsLight,
    /// Medium-tier neural TTS (e.g. Kokoro-82M).
    TtsMedium,
    /// Speech-to-text / dictation model (e.g. Moonshine).
    Stt,
    /// Voice cloning model (Phase 8).
    VoiceClone,
}

/// License information for a model requiring user acknowledgment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLicense {
    /// License name (e.g. "Apache-2.0").
    pub name: String,
    /// SPDX expression.
    pub spdx: String,
    /// Summary or full text of the license terms.
    pub terms: String,
    /// Whether this license has been accepted by the user.
    pub accepted: bool,
}

/// One downloadable or installed file belonging to a model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelFile {
    /// Local filename relative to the model directory.
    pub filename: String,
    /// Expected byte size.
    pub size_bytes: u64,
    /// Expected SHA-256 hex checksum.
    pub sha256: String,
    /// Source download URL.
    pub url: String,
}

/// Complete manifest definition for a model in the catalogue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelManifest {
    /// Stable model identifier.
    pub id: ModelId,
    /// Human-readable display name.
    pub name: String,
    /// Short description of the model.
    pub description: String,
    /// Category / purpose.
    pub kind: ModelKind,
    /// Author / organization.
    pub author: String,
    /// Version string.
    pub version: String,
    /// License information.
    pub license: ModelLicense,
    /// Files comprising this model.
    pub files: Vec<ModelFile>,
    /// Whether this model is built into the binary and needs no downloads.
    pub is_builtin: bool,
    /// Voices provided by this model.
    pub voice_names: Vec<String>,
}

impl ModelManifest {
    /// Total expected size of the model in bytes.
    #[must_use]
    pub fn total_size_bytes(&self) -> u64 {
        self.files.iter().map(|f| f.size_bytes).sum()
    }

    /// Check if all files exist and match their expected sizes in `model_dir`.
    #[must_use]
    pub fn is_installed_at(&self, model_dir: &Path) -> bool {
        if self.is_builtin {
            return true;
        }
        if self.files.is_empty() {
            return false;
        }
        for file in &self.files {
            let path = model_dir.join(&file.filename);
            match std::fs::metadata(path) {
                Ok(meta) => {
                    if meta.len() != file.size_bytes {
                        return false;
                    }
                }
                Err(_) => return false,
            }
        }
        true
    }
}

/// Installation status of a model on the local system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelStatus {
    /// Not downloaded or present locally.
    NotInstalled,
    /// Actively downloading with a progress ratio (0.0 to 1.0).
    Downloading { progress: f32 },
    /// Installed locally on disk.
    Installed { path: PathBuf },
    /// Installed and SHA-256 verified against manifest.
    Verified { path: PathBuf },
    /// Verification failed or corrupted file.
    Corrupt { reason: String },
}

impl ModelStatus {
    /// Returns true if installed and verified or ready to use.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Installed { .. } | Self::Verified { .. })
    }
}

/// Standard built-in catalogue of models for Audio Labs Pro.
#[must_use]
pub fn standard_catalogue() -> Vec<ModelManifest> {
    vec![
        ModelManifest {
            id: ModelId::new("builtin-acoustic"),
            name: "Built-in Acoustic TTS".into(),
            description: "Instant, zero-dependency embedded acoustic synthesizer for voice-over drafts and offline script previews.".into(),
            kind: ModelKind::TtsLight,
            author: "Audio Labs Pro".into(),
            version: "1.0.0".into(),
            license: ModelLicense {
                name: "Apache-2.0".into(),
                spdx: "Apache-2.0".into(),
                terms: "Permissive Apache 2.0 license. Free for commercial and non-commercial use.".into(),
                accepted: true,
            },
            files: vec![],
            is_builtin: true,
            voice_names: vec!["Aura (Clean)".into(), "Atlas (Warm)".into(), "Echo (Clear)".into(), "Sol (Bright)".into()],
        },
        ModelManifest {
            id: ModelId::new("kokoro-82m"),
            name: "Kokoro 82M".into(),
            description: "High quality 82M parameter English TTS neural model via ONNX Runtime.".into(),
            kind: ModelKind::TtsMedium,
            author: "Hexgrad / StyleTTS2".into(),
            version: "v0.19".into(),
            license: ModelLicense {
                name: "Apache-2.0".into(),
                spdx: "Apache-2.0".into(),
                terms: "Apache 2.0 License. Model weights and architecture are permissive for open-source and commercial applications.".into(),
                accepted: false,
            },
            files: vec![
                ModelFile {
                    filename: "model.onnx".into(),
                    size_bytes: 341_835_776,
                    sha256: "b49a15f02bc7b0c95333fdf081fa4bc89f5bc3a67d512a297926e25f8d95e7c8".into(),
                    url: "https://huggingface.co/hexgrad/Kokoro-82M/resolve/main/kokoro-v0_19.onnx".into(),
                },
                ModelFile {
                    filename: "voices.bin".into(),
                    size_bytes: 1_048_576,
                    sha256: "06b04e6e6dcff9a28c3065b2149b068da8fe59da376a9118c7bf1fa96ca7d930".into(),
                    url: "https://huggingface.co/hexgrad/Kokoro-82M/resolve/main/voices.bin".into(),
                },
            ],
            is_builtin: false,
            voice_names: vec!["Heart (Natural)".into(), "Bella (Expressive)".into(), "Michael (Narration)".into()],
        },
        ModelManifest {
            id: ModelId::new("kitten-nano"),
            name: "Kitten Nano".into(),
            description: "Ultra-compact neural speech synthesizer for fast, low-latency CPU generation.".into(),
            kind: ModelKind::TtsLight,
            author: "Audio Labs Pro Community".into(),
            version: "0.2.1".into(),
            license: ModelLicense {
                name: "Apache-2.0".into(),
                spdx: "Apache-2.0".into(),
                terms: "Apache 2.0 License.".into(),
                accepted: false,
            },
            files: vec![
                ModelFile {
                    filename: "kitten_nano.onnx".into(),
                    size_bytes: 42_500_000,
                    sha256: "5d41402abc4b2a76b9719d911017c592b23a9d70eb0e3ec8b8b5e2d67d71b764".into(),
                    url: "https://models.audiolabspro.internal/kitten-nano.onnx".into(),
                },
            ],
            is_builtin: false,
            voice_names: vec!["Kitten Fast (Light)".into()],
        },
        ModelManifest {
            id: ModelId::new("moonshine-tiny"),
            name: "Moonshine Tiny (ASR)".into(),
            description: "Permissively licensed (MIT) CPU-friendly fast English speech-to-text model.".into(),
            kind: ModelKind::Stt,
            author: "Useful Sensors".into(),
            version: "1.0".into(),
            license: ModelLicense {
                name: "MIT".into(),
                spdx: "MIT".into(),
                terms: "MIT License. Permissive for open-source and commercial applications.".into(),
                accepted: false,
            },
            files: vec![
                ModelFile {
                    filename: "encoder.onnx".into(),
                    size_bytes: 84_000_000,
                    sha256: "c830dbfa62243e8bb58c8ecb164a66a7bcf1489e81b65d5642e47c17d7b27ea3".into(),
                    url: "https://huggingface.co/onnx-community/moonshine-tiny-ONNX/resolve/main/encoder_model.onnx".into(),
                },
            ],
            is_builtin: false,
            voice_names: vec![],
        },
    ]
}
