//! Qwen3-TTS 0.6B neural voice cloning (ADR 0008, ADR 0018, ADR 0029).
//!
//! The official Apache-2.0 Safetensors checkpoint is executed by `crane-core`'s
//! Candle implementation. Voice cloning uses in-context learning (ICL): the
//! stored reference recording and transcript condition the speaker encoder,
//! talker, code predictor, and native 12 Hz speech tokenizer. The lightweight
//! acoustic clone tier remains independent; its 256-value voiceprint is never
//! presented to Qwen as a neural speaker embedding.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use alp_media::{PcmBuffer, decode, resample};
use alp_voice::profile::{VoiceParams, VoiceProfile};
use alp_voice::{CloneBackend, TtsBackend, TtsRequest, VoiceError, VoiceInfo, VoiceKind};
use crane_core::generation::SpeechOptions;
use crane_core::models::qwen3_tts::Model;
use crane_core::models::{DType, Device};

/// Qwen3-TTS operates at 24 kHz mono output.
pub const SAMPLE_RATE: u32 = 24_000;

/// Catalogue id of the model this engine executes.
pub const MODEL_ID: &str = "qwen3-tts";

/// Official talker, speaker-encoder and code-predictor checkpoint.
const TALKER_FILE: &str = "model.safetensors";

/// Official 12 Hz speech-tokenizer checkpoint.
const DECODER_FILE: &str = "speech_tokenizer/model.safetensors";

/// Files required by the official Candle model loader.
const AUXILIARY_FILES: [&str; 5] = [
    "config.json",
    "tokenizer_config.json",
    "vocab.json",
    "merges.txt",
    "speech_tokenizer/config.json",
];

/// A loaded Qwen3-TTS engine. The mutex serializes mutable generation state
/// and also makes failures on a poisoned lock explicit rather than panicking.
pub struct Qwen3Engine {
    model_dir: PathBuf,
    model: Mutex<Model>,
}

impl std::fmt::Debug for Qwen3Engine {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Qwen3Engine")
            .field("model_dir", &self.model_dir)
            .finish_non_exhaustive()
    }
}

impl Qwen3Engine {
    /// Load the official Qwen3-TTS installation into CPU memory.
    pub fn load(model_dir: &Path) -> Result<Self, VoiceError> {
        for file in [TALKER_FILE, DECODER_FILE] {
            let path = model_dir.join(file);
            if !path.is_file() {
                return Err(VoiceError::Unavailable(format!(
                    "Qwen3-TTS weights not found: {}; run `alp-cli model download qwen3-tts` to install them",
                    path.display()
                )));
            }
        }
        for file in AUXILIARY_FILES {
            let path = model_dir.join(file);
            if !path.is_file() {
                return Err(VoiceError::Unavailable(format!(
                    "Qwen3-TTS asset not found: {}; run `alp-cli model download qwen3-tts` to install it",
                    path.display()
                )));
            }
        }
        let model_path = model_dir.to_str().ok_or_else(|| {
            VoiceError::Backend("Qwen3-TTS model directory is not valid UTF-8".into())
        })?;
        let model = Model::new(model_path, &Device::Cpu, &DType::F32)
            .map_err(|error| VoiceError::Backend(format!("loading Qwen3-TTS: {error}")))?;
        let output_rate = model.sample_rate();
        if output_rate != SAMPLE_RATE {
            return Err(VoiceError::Backend(format!(
                "Qwen3-TTS returned an unsupported output rate {output_rate}, expected {SAMPLE_RATE}"
            )));
        }
        Ok(Self {
            model_dir: model_dir.to_path_buf(),
            model: Mutex::new(model),
        })
    }

    /// Access the directory containing the model weights.
    #[must_use]
    pub fn model_dir(&self) -> &Path {
        &self.model_dir
    }

    /// Path of the official combined talker checkpoint.
    #[must_use]
    pub fn talker_path(&self) -> PathBuf {
        self.model_dir.join(TALKER_FILE)
    }

    /// Path of the official speech-tokenizer checkpoint.
    #[must_use]
    pub fn decoder_path(&self) -> PathBuf {
        self.model_dir.join(DECODER_FILE)
    }

    /// Read a stored reference and produce the mono samples required by Crane.
    fn read_reference(path: &Path, target_rate: u32) -> Result<PcmBuffer, VoiceError> {
        let decoded = decode(path).map_err(|error| {
            VoiceError::Backend(format!(
                "could not decode Qwen3-TTS reference audio {}: {error}",
                path.display()
            ))
        })?;
        let converted = resample(&decoded, target_rate)
            .map_err(|error| VoiceError::Backend(format!("resampling reference audio: {error}")))?;
        let channels = usize::from(converted.channels());
        let mono = converted
            .samples()
            .chunks_exact(channels)
            .map(|frame| frame.iter().sum::<f32>() / channels as f32)
            .collect();
        PcmBuffer::new(target_rate, 1, mono)
            .map_err(|error| VoiceError::Backend(format!("normalizing reference audio: {error}")))
    }

    fn neural_options(text: &str) -> SpeechOptions {
        SpeechOptions {
            // Short requests do not allocate unnecessary codec frames; longer
            // chunks retain headroom without making the IPC response unbounded.
            max_new_tokens: text.len().saturating_mul(8).clamp(128, 8_192),
            temperature: 0.9,
            top_p: Some(1.0),
            repetition_penalty: 1.05,
            ..SpeechOptions::default()
        }
    }
}

impl TtsBackend for Qwen3Engine {
    fn id(&self) -> &str {
        MODEL_ID
    }

    fn voices(&self) -> Vec<VoiceInfo> {
        vec![VoiceInfo {
            id: "qwen3-clone".into(),
            name: "Qwen3 Dynamic Clone".into(),
            language: "en".into(),
            kind: VoiceKind::Cloned,
        }]
    }

    fn synthesize(&self, _request: &TtsRequest) -> Result<PcmBuffer, VoiceError> {
        Err(VoiceError::Backend(
            "the Qwen3-TTS Base model requires a cloned voice profile with reference audio and its transcript; plain synthesis is not available"
                .into(),
        ))
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
                "reference audio is empty; cannot create a neural clone".into(),
            ));
        }
        let transcript = transcript
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .ok_or_else(|| {
                VoiceError::Backend(
                    "Qwen3-TTS ICL cloning requires the reference audio transcript".into(),
                )
            })?;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        Ok(VoiceProfile {
            id: format!("qwen3-clone-{timestamp}"),
            name: "Qwen3 Cloned Voice".into(),
            kind: VoiceKind::Cloned,
            backend: MODEL_ID.into(),
            // As with the acoustic GUI flow, the human enrollment code stores
            // the reference under this name after it attests consent.
            reference_audio: Some(PathBuf::from("reference.wav")),
            transcript: Some(transcript.to_owned()),
            // ICL conditions from the original PCM, not the incompatible
            // 256-value acoustic descriptor.
            cached_embedding: None,
            tags: vec!["cloned".into(), "qwen3-tts".into(), "neural".into()],
            default_params: VoiceParams::default(),
            created_at: timestamp,
            // Safety rule: human consent must be attested before saving.
            consent: None,
        })
    }

    fn synthesize_with(
        &self,
        profile: &VoiceProfile,
        request: &TtsRequest,
    ) -> Result<PcmBuffer, VoiceError> {
        if request.text.trim().is_empty() {
            return Err(VoiceError::Backend("Qwen3-TTS target text is empty".into()));
        }
        let reference_path = profile.reference_audio.as_deref().ok_or_else(|| {
            VoiceError::Backend(
                "Qwen3-TTS profile has no reference audio; re-enroll or import its reference.wav"
                    .into(),
            )
        })?;
        let reference_transcript = profile
            .transcript
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .ok_or_else(|| {
                VoiceError::Backend(
                    "Qwen3-TTS profile has no reference transcript; ICL cloning requires it".into(),
                )
            })?;
        let mut model = self
            .model
            .lock()
            .map_err(|_| VoiceError::Backend("Qwen3-TTS model lock is poisoned".into()))?;
        let reference = Self::read_reference(reference_path, model.speaker_encoder_sample_rate())?;
        if reference.samples().is_empty() {
            return Err(VoiceError::Backend(
                "Qwen3-TTS reference audio is empty".into(),
            ));
        }
        let (audio, sample_rate) = model
            .generate_voice_clone(
                request.text.trim(),
                "english",
                reference.samples(),
                reference_transcript,
                &Self::neural_options(&request.text),
            )
            .map_err(|error| {
                VoiceError::Backend(format!("Qwen3-TTS generation failed: {error}"))
            })?;
        let samples = audio
            .to_dtype(DType::F32)
            .and_then(|tensor| tensor.flatten_all())
            .and_then(|tensor| tensor.to_vec1::<f32>())
            .map_err(|error| VoiceError::Backend(format!("decoding Qwen3-TTS audio: {error}")))?;
        if samples.is_empty() || !samples.iter().all(|sample| sample.is_finite()) {
            return Err(VoiceError::Backend(
                "Qwen3-TTS generated no finite audio samples".into(),
            ));
        }
        let output = PcmBuffer::new(sample_rate, 1, samples)
            .map_err(|error| VoiceError::Backend(format!("invalid Qwen3-TTS output: {error}")))?;
        if output.rms() <= 1.0e-5 {
            return Err(VoiceError::Backend(
                "Qwen3-TTS generated silent output".into(),
            ));
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_on_empty_dir_returns_unavailable_with_download_command() {
        let dir = tempfile::tempdir().expect("tempdir");
        let error = Qwen3Engine::load(dir.path()).expect_err("expected failure on empty dir");
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
        std::fs::write(dir.path().join(TALKER_FILE), b"not weights").expect("talker");
        let error = Qwen3Engine::load(dir.path()).expect_err("must refuse");
        let message = error.to_string();
        assert!(
            message.contains("speech_tokenizer/model.safetensors"),
            "{message}"
        );
    }

    #[test]
    fn corrupt_safetensors_never_construct_an_engine() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join("speech_tokenizer")).expect("tokenizer dir");
        for file in [TALKER_FILE, DECODER_FILE] {
            std::fs::write(dir.path().join(file), b"not weights").expect("fixture");
        }
        for file in AUXILIARY_FILES {
            std::fs::write(dir.path().join(file), b"{}").expect("fixture");
        }
        let error = Qwen3Engine::load(dir.path()).expect_err("must refuse");
        assert!(matches!(error, VoiceError::Backend(_)), "{error:?}");
    }

    #[test]
    fn reference_audio_is_downmixed_and_resampled() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("reference.wav");
        alp_media::write_test_tone(&path, 1.0, 48_000, 2, 440.0, 0.25).expect("reference");
        let normalized = Qwen3Engine::read_reference(&path, SAMPLE_RATE).expect("normalized");
        assert_eq!(normalized.sample_rate(), SAMPLE_RATE);
        assert_eq!(normalized.channels(), 1);
        assert!((normalized.frames() as i64 - 24_000).abs() <= 2);
        assert!(normalized.rms() > 0.1);
    }

    #[test]
    fn generation_options_cover_short_and_long_requests() {
        assert_eq!(Qwen3Engine::neural_options("Hello.").max_new_tokens, 128);
        let long = "word ".repeat(500);
        assert_eq!(Qwen3Engine::neural_options(&long).max_new_tokens, 8_192);
    }

    /// Requires explicit official assets; ordinary test runs do no downloads.
    #[test]
    fn official_model_generates_neural_audio_when_installed() {
        let Ok(model_dir) = std::env::var("ALP_QWEN3_DIR") else {
            eprintln!("skip: set ALP_QWEN3_DIR to the official Qwen3-TTS model directory");
            return;
        };
        let Ok(reference_path) = std::env::var("ALP_QWEN3_REFERENCE") else {
            eprintln!("skip: set ALP_QWEN3_REFERENCE to the matching reference WAV");
            return;
        };
        let engine = Qwen3Engine::load(Path::new(&model_dir)).expect("load real model");
        let reference = alp_media::decode(reference_path).expect("reference");
        let mut profile = engine
            .create_profile(&reference, Some("Okay. Yeah. I resent you. I love you. I respect you. But you know what? You blew it! And thanks to you."))
            .expect("profile");
        profile.reference_audio = Some(std::env::var("ALP_QWEN3_REFERENCE").expect("path").into());
        let output = engine
            .synthesize_with(
                &profile,
                &TtsRequest {
                    text: "Hello world. This is a neural voice cloning test.".into(),
                    voice_id: profile.id.clone(),
                    speed: 1.0,
                },
            )
            .expect("real neural voice cloning");
        assert_eq!(output.sample_rate(), SAMPLE_RATE);
        assert_eq!(output.channels(), 1);
        assert!(output.rms() > 1.0e-5);
        assert!(output.samples().iter().all(|sample| sample.is_finite()));
    }
}
