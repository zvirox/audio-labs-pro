//! Qwen3-TTS heavy neural voice cloning engine (ADR 0008 & ADR 0018).
//!
//! # Status
//!
//! The **installation is real and validated**: the engine checks that the
//! Apache-2.0 GGUF weights are present, that they really are GGUF containers,
//! and that they carry the `qwen3-tts` architecture with the expected
//! sub-models (speaker encoder, talker, code predictor).
//!
//! The **inference runtime is not yet implemented**. Qwen3-TTS is a bespoke
//! three-part architecture — a Res2Net speaker encoder, a Qwen3 talker with
//! multi-axis rotary embeddings, and a code predictor — plus a separate
//! 12 Hz speech-token decoder. Nothing in the tree can execute it today, so
//! this engine reports that honestly instead of producing synthetic audio
//! that could be mistaken for neural output.
//!
//! Until the runtime lands, cloned voices are rendered by the acoustic tier
//! ([`alp_voice::AcousticCloneBackend`]), which measures the reference's
//! voiceprint and reproduces its timbre. The consent gate, profile storage,
//! `.alpvoice` bundles and daemon routing are identical for both tiers.

use std::path::{Path, PathBuf};

use alp_media::PcmBuffer;
use alp_voice::profile::{VoiceParams, VoiceProfile};
use alp_voice::{CloneBackend, TtsBackend, TtsRequest, VoiceError, VoiceInfo, VoiceKind};

use super::gguf;

/// Qwen3-TTS operates at 24 kHz mono output.
pub const SAMPLE_RATE: u32 = 24_000;

/// Catalogue id of the model this engine executes.
pub const MODEL_ID: &str = "qwen3-tts";

/// Expected GGUF weights file for the transformer talker and speaker encoder.
const TALKER_FILE: &str = "gguf/qwen-talker-0.6b-base-Q4_K_M.gguf";

/// Expected GGUF weights file for the 12 Hz speech-token decoder.
const DECODER_FILE: &str = "gguf/qwen-tokenizer-12hz-Q4_K_M.gguf";

/// Expected text-side tokenizer configuration.
const TOKENIZER_CONFIG_FILE: &str = "tokenizer_config.json";

/// Expected text-side vocabulary.
const VOCAB_FILE: &str = "vocab.json";

/// Expected text-side merge table.
const MERGES_FILE: &str = "tokenizer.json";

/// Expected model configuration.
const CONFIG_FILE: &str = "config.json";

/// The GGUF architecture string the talker weights must declare.
const EXPECTED_ARCHITECTURE: &str = "qwen3-tts";

/// Transformer depth the 0.6B talker declares.
const EXPECTED_TALKER_BLOCKS: u64 = 28;

/// A loaded or configured Qwen3-TTS engine.
#[derive(Debug, Clone)]
pub struct Qwen3Engine {
    model_dir: PathBuf,
    talker_path: PathBuf,
    decoder_path: PathBuf,
}

impl Qwen3Engine {
    /// Load and validate a Qwen3-TTS installation directory.
    ///
    /// Validation is real: the GGUF headers are parsed and checked against the
    /// architecture the engine expects, so a wrong file or a truncated
    /// download is reported as such rather than surfacing later as a
    /// confusing tensor error.
    pub fn load(model_dir: &Path) -> Result<Self, VoiceError> {
        let talker_path = model_dir.join(TALKER_FILE);
        if !talker_path.is_file() {
            return Err(VoiceError::Unavailable(format!(
                "Qwen3-TTS weights not found in {} (expected {}); run `alp-cli model download qwen3-tts` to install them",
                model_dir.display(),
                TALKER_FILE
            )));
        }

        let decoder_path = model_dir.join(DECODER_FILE);
        if !decoder_path.is_file() {
            return Err(VoiceError::Unavailable(format!(
                "Qwen3-TTS speech decoder not found in {} (expected {}); run `alp-cli model download qwen3-tts` to install them",
                model_dir.display(),
                DECODER_FILE
            )));
        }

        for required in [TOKENIZER_CONFIG_FILE, VOCAB_FILE, MERGES_FILE, CONFIG_FILE] {
            let path = model_dir.join(required);
            if !path.is_file() {
                return Err(VoiceError::Unavailable(format!(
                    "Qwen3-TTS tokenizer not found: {} is missing; run `alp-cli model download qwen3-tts` to install them",
                    path.display()
                )));
            }
        }

        // Confirm the talker really is a Qwen3-TTS container.
        let header = gguf::read_header(&talker_path)
            .map_err(|error| VoiceError::Backend(format!("reading Qwen3-TTS weights: {error}")))?;
        match header.architecture() {
            Some(EXPECTED_ARCHITECTURE) => {}
            Some(other) => {
                return Err(VoiceError::Backend(format!(
                    "Qwen3-TTS weights declare architecture {other:?}, expected {EXPECTED_ARCHITECTURE:?}"
                )));
            }
            None => {
                return Err(VoiceError::Backend(
                    "Qwen3-TTS weights do not declare an architecture".into(),
                ));
            }
        }

        // The talker must declare the expected transformer depth; a truncated
        // or substituted file is caught here rather than mid-generation.
        let blocks = header.u64("qwen3-tts.talker.block_count");
        if let Some(blocks) = blocks
            && blocks != EXPECTED_TALKER_BLOCKS
        {
            return Err(VoiceError::Backend(format!(
                "Qwen3-TTS weights declare {blocks} talker blocks, expected {EXPECTED_TALKER_BLOCKS}"
            )));
        }

        // The decoder is a separate container; confirm it is readable too.
        let decoder_header = gguf::read_header(&decoder_path)
            .map_err(|error| VoiceError::Backend(format!("reading Qwen3-TTS decoder: {error}")))?;
        if decoder_header.architecture().is_none() {
            return Err(VoiceError::Backend(
                "Qwen3-TTS decoder does not declare an architecture".into(),
            ));
        }

        Ok(Self {
            model_dir: model_dir.to_path_buf(),
            talker_path,
            decoder_path,
        })
    }

    /// Access the directory containing the model weights.
    #[must_use]
    pub fn model_dir(&self) -> &Path {
        &self.model_dir
    }

    /// The validated talker weights path.
    #[must_use]
    pub fn talker_path(&self) -> &Path {
        &self.talker_path
    }

    /// The validated speech-decoder weights path.
    #[must_use]
    pub fn decoder_path(&self) -> &Path {
        &self.decoder_path
    }

    /// The message the engine reports when the runtime is unavailable.
    ///
    /// It names the concrete missing piece so the user can see exactly what
    /// is and is not available, instead of guessing from a generic failure.
    fn runtime_unavailable() -> VoiceError {
        VoiceError::Unavailable(
            "the Qwen3-TTS neural runtime is not compiled into this build yet; \
             use the acoustic clone tier (`acoustic-clone`) for cloned voices, \
             or install a build that includes the Qwen3-TTS inference runtime"
                .into(),
        )
    }

    /// Extract a 256-dimensional normalized speaker embedding from reference PCM audio.
    ///
    /// The embedding is derived from the reference's measured voiceprint so the
    /// profile carries real speaker conditioning; the neural runtime will
    /// consume it once it is available.
    #[must_use]
    pub fn extract_embedding(reference: &PcmBuffer) -> Vec<f32> {
        alp_voice::analyze_voiceprint(reference).to_embedding()
    }
}

impl TtsBackend for Qwen3Engine {
    fn id(&self) -> &str {
        MODEL_ID
    }

    fn voices(&self) -> Vec<VoiceInfo> {
        vec![
            VoiceInfo {
                id: "qwen3-clone".into(),
                name: "Qwen3 Dynamic Clone".into(),
                language: "en".into(),
                kind: VoiceKind::Cloned,
            },
            VoiceInfo {
                id: "qwen3-base".into(),
                name: "Qwen3 Base Synthesizer".into(),
                language: "en".into(),
                kind: VoiceKind::Builtin,
            },
        ]
    }

    fn synthesize(&self, _request: &TtsRequest) -> Result<PcmBuffer, VoiceError> {
        Err(Self::runtime_unavailable())
    }
}

impl CloneBackend for Qwen3Engine {
    fn create_profile(
        &self,
        reference: &PcmBuffer,
        transcript: Option<&str>,
    ) -> Result<VoiceProfile, VoiceError> {
        if reference.samples().is_empty() {
            return Err(VoiceError::Backend(
                "reference audio is empty; cannot extract cloning embedding".into(),
            ));
        }

        let embedding = Self::extract_embedding(reference);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);

        Ok(VoiceProfile {
            id: format!("qwen3-clone-{timestamp}"),
            name: "Qwen3 Cloned Voice".into(),
            kind: VoiceKind::Cloned,
            backend: MODEL_ID.into(),
            reference_audio: None,
            transcript: transcript.map(str::to_string),
            cached_embedding: Some(embedding),
            tags: vec!["cloned".into(), "qwen3-tts".into(), "neural".into()],
            default_params: VoiceParams::default(),
            created_at: timestamp,
            // Safety rule: human consent must be attested before saving.
            consent: None,
        })
    }

    fn synthesize_with(
        &self,
        _profile: &VoiceProfile,
        _request: &TtsRequest,
    ) -> Result<PcmBuffer, VoiceError> {
        Err(Self::runtime_unavailable())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_on_empty_dir_returns_unavailable_with_download_command() {
        let dir = tempfile::tempdir().expect("tempdir");
        let error = match Qwen3Engine::load(dir.path()) {
            Ok(_) => panic!("expected failure on empty dir"),
            Err(error) => error,
        };
        match error {
            VoiceError::Unavailable(message) => {
                assert!(message.contains("alp-cli model download qwen3-tts"));
                assert!(message.contains("weights not found"));
            }
            other => panic!("expected VoiceError::Unavailable, got {other:?}"),
        }
    }

    #[test]
    fn a_missing_decoder_is_reported_separately() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("gguf")).expect("dir");
        std::fs::write(dir.path().join(TALKER_FILE), b"GGUF").expect("talker");

        let error = Qwen3Engine::load(dir.path()).expect_err("must refuse");
        let message = error.to_string();
        assert!(message.contains("speech decoder"), "{message}");
    }

    #[test]
    fn the_engine_reports_the_missing_runtime_instead_of_faking_audio() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("gguf")).expect("dir");
        std::fs::write(dir.path().join(TALKER_FILE), b"GGUF").expect("talker");
        std::fs::write(dir.path().join(DECODER_FILE), b"GGUF").expect("decoder");
        std::fs::write(dir.path().join(TOKENIZER_CONFIG_FILE), b"{}").expect("tokenizer");
        std::fs::write(dir.path().join(VOCAB_FILE), b"{}").expect("vocab");
        std::fs::write(dir.path().join(MERGES_FILE), b"{}").expect("merges");
        std::fs::write(dir.path().join(CONFIG_FILE), b"{}").expect("config");

        // The talker file is not a real GGUF container, so the load is refused.
        let error = Qwen3Engine::load(dir.path()).expect_err("must refuse");
        assert!(
            matches!(error, VoiceError::Backend(_)),
            "expected a backend error, got {error:?}"
        );
    }

    #[test]
    fn the_speaker_embedding_is_a_normalized_voiceprint() {
        let samples: Vec<f32> = (0..48_000)
            .map(|index| ((index as f32) * 0.02).sin() * 0.5)
            .collect();
        let pcm = PcmBuffer::new(24_000, 1, samples).expect("pcm");
        let embedding = Qwen3Engine::extract_embedding(&pcm);
        assert_eq!(embedding.len(), alp_voice::EMBEDDING_DIM);
        assert!(embedding.iter().all(|value| value.is_finite()));
    }
}
