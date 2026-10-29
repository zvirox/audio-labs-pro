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
            author: "hexgrad / onnx-community".into(),
            version: "v1.0".into(),
            license: ModelLicense {
                name: "Apache-2.0".into(),
                spdx: "Apache-2.0".into(),
                terms: "Apache 2.0 License. Model weights and architecture are permissive for open-source and commercial applications.".into(),
                accepted: false,
            },
            files: vec![
                ModelFile {
                    filename: "onnx/model.onnx".into(),
                    size_bytes: 325_532_232,
                    sha256: "8fbea51ea711f2af382e88c833d9e288c6dc82ce5e98421ea61c058ce21a34cb".into(),
                    url: "https://huggingface.co/onnx-community/Kokoro-82M-v1.0-ONNX/resolve/main/onnx/model.onnx".into(),
                },
                ModelFile {
                    filename: "tokenizer.json".into(),
                    size_bytes: 3_497,
                    sha256: "77a02c8e164413299b4b4c403b14f8e0e1c1b727db4d46a09d6327b861060a34".into(),
                    url: "https://huggingface.co/onnx-community/Kokoro-82M-v1.0-ONNX/resolve/main/tokenizer.json".into(),
                },
                ModelFile {
                    filename: "voices/af_heart.bin".into(),
                    size_bytes: 522_240,
                    sha256: "d583ccff3cdca2f7fae535cb998ac07e9fcb90f09737b9a41fa2734ec44a8f0b".into(),
                    url: "https://huggingface.co/onnx-community/Kokoro-82M-v1.0-ONNX/resolve/main/voices/af_heart.bin".into(),
                },
                ModelFile {
                    filename: "voices/af_bella.bin".into(),
                    size_bytes: 522_240,
                    sha256: "f69d836209b78eb8c66e75e3cda491e26ea838a3674257e9d4e5703cbaf55c8b".into(),
                    url: "https://huggingface.co/onnx-community/Kokoro-82M-v1.0-ONNX/resolve/main/voices/af_bella.bin".into(),
                },
                ModelFile {
                    filename: "voices/am_michael.bin".into(),
                    size_bytes: 522_240,
                    sha256: "1d1f21dd8da39c30705cd4c75d039d265e9bc4a2a93ed09bc9e1b1225eb95ba1".into(),
                    url: "https://huggingface.co/onnx-community/Kokoro-82M-v1.0-ONNX/resolve/main/voices/am_michael.bin".into(),
                },
            ],
            is_builtin: false,
            voice_names: vec!["Heart (US female)".into(), "Bella (US female)".into(), "Michael (US male)".into()],
        },
        ModelManifest {
            id: ModelId::new("kitten-nano"),
            name: "Kitten Nano".into(),
            description: "Ultra-compact (~24 MB quantized) neural speech synthesizer for fast, low-latency CPU generation.".into(),
            kind: ModelKind::TtsLight,
            author: "KittenML / onnx-community".into(),
            version: "0.1".into(),
            license: ModelLicense {
                name: "Apache-2.0".into(),
                spdx: "Apache-2.0".into(),
                terms: "Apache 2.0 License. Model weights and architecture are permissive for open-source and commercial applications.".into(),
                accepted: false,
            },
            files: vec![
                ModelFile {
                    filename: "onnx/model_quantized.onnx".into(),
                    size_bytes: 23_792_492,
                    sha256: "9f2824641956d6640b351b13fe8099293bc00c1a041e852157760ff5292fc28e".into(),
                    url: "https://huggingface.co/onnx-community/kitten-tts-nano-0.1-ONNX/resolve/main/onnx/model_quantized.onnx".into(),
                },
                ModelFile {
                    filename: "tokenizer.json".into(),
                    size_bytes: 4_468,
                    sha256: "3c6b9859e4dd845cbc5ff4bc6de4b6fd9a03fdb28bc2d657219db4a4f1f9907b".into(),
                    url: "https://huggingface.co/onnx-community/kitten-tts-nano-0.1-ONNX/resolve/main/tokenizer.json".into(),
                },
                ModelFile {
                    filename: "tokenizer_config.json".into(),
                    size_bytes: 113,
                    sha256: "be1cb066d6ef6b074b3f15e6a6dd21ac88ff3cdaedf325f0aaed686c70f75d20".into(),
                    url: "https://huggingface.co/onnx-community/kitten-tts-nano-0.1-ONNX/resolve/main/tokenizer_config.json".into(),
                },
                ModelFile {
                    filename: "config.json".into(),
                    size_bytes: 97,
                    sha256: "87f42506cb9c37a5d21ead1fbe4836920ac6e1d2b531a8ba3570398578bd061d".into(),
                    url: "https://huggingface.co/onnx-community/kitten-tts-nano-0.1-ONNX/resolve/main/config.json".into(),
                },
                ModelFile {
                    filename: "voices/expr-voice-2-f.bin".into(),
                    size_bytes: 1_024,
                    sha256: "908f82110142048fd6bf66cc494ca07c1c459b3eb9324fd55a70e5f8d02ecc0a".into(),
                    url: "https://huggingface.co/onnx-community/kitten-tts-nano-0.1-ONNX/resolve/main/voices/expr-voice-2-f.bin".into(),
                },
                ModelFile {
                    filename: "voices/expr-voice-2-m.bin".into(),
                    size_bytes: 1_024,
                    sha256: "92c290fdef55b96fa2521376bbdaf6fa4e4018e95c7479d674e0c18109a0866a".into(),
                    url: "https://huggingface.co/onnx-community/kitten-tts-nano-0.1-ONNX/resolve/main/voices/expr-voice-2-m.bin".into(),
                },
                ModelFile {
                    filename: "voices/expr-voice-3-f.bin".into(),
                    size_bytes: 1_024,
                    sha256: "fa058a4f77a385dbafa15bfde52c3b0d223463a60d852787df3305a5b7409583".into(),
                    url: "https://huggingface.co/onnx-community/kitten-tts-nano-0.1-ONNX/resolve/main/voices/expr-voice-3-f.bin".into(),
                },
                ModelFile {
                    filename: "voices/expr-voice-3-m.bin".into(),
                    size_bytes: 1_024,
                    sha256: "75cafcedb1d513df0e79369b18533cc02d07d5b1f0bad00628aa35d8200575a5".into(),
                    url: "https://huggingface.co/onnx-community/kitten-tts-nano-0.1-ONNX/resolve/main/voices/expr-voice-3-m.bin".into(),
                },
                ModelFile {
                    filename: "voices/expr-voice-4-f.bin".into(),
                    size_bytes: 1_024,
                    sha256: "986600f5065e7b1927c68494f0b3760e96b979ae5dd97f2e947432ed42ede20e".into(),
                    url: "https://huggingface.co/onnx-community/kitten-tts-nano-0.1-ONNX/resolve/main/voices/expr-voice-4-f.bin".into(),
                },
                ModelFile {
                    filename: "voices/expr-voice-4-m.bin".into(),
                    size_bytes: 1_024,
                    sha256: "05de675b434edda9b692529e53c42ca749123358fc8e21a26e945bf629153d91".into(),
                    url: "https://huggingface.co/onnx-community/kitten-tts-nano-0.1-ONNX/resolve/main/voices/expr-voice-4-m.bin".into(),
                },
                ModelFile {
                    filename: "voices/expr-voice-5-f.bin".into(),
                    size_bytes: 1_024,
                    sha256: "a849c8115b266d415390d419673a0a0bdbc77f5142403ebed9f4ab07fbe7ad05".into(),
                    url: "https://huggingface.co/onnx-community/kitten-tts-nano-0.1-ONNX/resolve/main/voices/expr-voice-5-f.bin".into(),
                },
                ModelFile {
                    filename: "voices/expr-voice-5-m.bin".into(),
                    size_bytes: 1_024,
                    sha256: "4133158853e381c66fabd37eca8e1650ed934b7b3e68c32d9db8ea745dfa00a6".into(),
                    url: "https://huggingface.co/onnx-community/kitten-tts-nano-0.1-ONNX/resolve/main/voices/expr-voice-5-m.bin".into(),
                },
            ],
            is_builtin: false,
            voice_names: vec![
                "Expr Voice 2 (F)".into(),
                "Expr Voice 2 (M)".into(),
                "Expr Voice 3 (F)".into(),
                "Expr Voice 3 (M)".into(),
                "Expr Voice 4 (F)".into(),
                "Expr Voice 4 (M)".into(),
                "Expr Voice 5 (F)".into(),
                "Expr Voice 5 (M)".into(),
            ],
        },
        ModelManifest {
            id: ModelId::new("moonshine-tiny"),
            name: "Moonshine Tiny (ASR)".into(),
            description: "Permissively licensed (MIT) CPU-friendly fast English speech-to-text model.".into(),
            kind: ModelKind::Stt,
            author: "Useful Sensors / onnx-community".into(),
            version: "1.0".into(),
            license: ModelLicense {
                name: "MIT".into(),
                spdx: "MIT".into(),
                terms: "MIT License. Permissive for open-source and commercial applications.".into(),
                accepted: false,
            },
            files: vec![
                ModelFile {
                    filename: "onnx/encoder_model.onnx".into(),
                    size_bytes: 30_882_331,
                    sha256: "cbbf580f703b2af2137e0f6d14cd87f31cc67bd858bfd8715403a9489982d1a5".into(),
                    url: "https://huggingface.co/onnx-community/moonshine-tiny-ONNX/resolve/main/onnx/encoder_model.onnx".into(),
                },
                ModelFile {
                    filename: "onnx/decoder_model_merged.onnx".into(),
                    size_bytes: 78_227_550,
                    sha256: "4131cef00b62942e9cdef691101f2cc7dbbcd828d71eee8c6c46c28fd051d6cb".into(),
                    url: "https://huggingface.co/onnx-community/moonshine-tiny-ONNX/resolve/main/onnx/decoder_model_merged.onnx".into(),
                },
                ModelFile {
                    filename: "config.json".into(),
                    size_bytes: 921,
                    sha256: "558e1e02069137c796ace1e50c48d8fe451f04a295929138e6bea885517f0edb".into(),
                    url: "https://huggingface.co/onnx-community/moonshine-tiny-ONNX/resolve/main/config.json".into(),
                },
                ModelFile {
                    filename: "generation_config.json".into(),
                    size_bytes: 147,
                    sha256: "f9b3f711b57be7def2e50a8942f64f36ee0a55fad5b84ff93a687b6c5bcc1d44".into(),
                    url: "https://huggingface.co/onnx-community/moonshine-tiny-ONNX/resolve/main/generation_config.json".into(),
                },
                ModelFile {
                    filename: "preprocessor_config.json".into(),
                    size_bytes: 128,
                    sha256: "fa43a7017ef85cd1d0fba0d9aae77c8adb16990ae6f11115631f41ec5d8aa679".into(),
                    url: "https://huggingface.co/onnx-community/moonshine-tiny-ONNX/resolve/main/preprocessor_config.json".into(),
                },
                ModelFile {
                    filename: "tokenizer.json".into(),
                    size_bytes: 3_761_754,
                    sha256: "7b913404bdd039af4756783218af4440bc07fb7d6d8258d677e34f95b3ec416f".into(),
                    url: "https://huggingface.co/onnx-community/moonshine-tiny-ONNX/resolve/main/tokenizer.json".into(),
                },
            ],
            is_builtin: false,
            voice_names: vec![],
        },
    ]
}
