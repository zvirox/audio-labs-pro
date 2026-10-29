//! Kokoro-82M neural TTS engine executed through ONNX Runtime.
//!
//! Lifted from the Phase 0 spike (`spikes/model-spike`, ADR 0008/0010): an
//! `ort` 2.0.0-rc.15 CPU session over the v1.0 fp32 ONNX export, an
//! MIT-licensed G2P front-end (`piper-plus-g2p`, no espeak-ng) that emits IPA,
//! a character vocabulary parsed from the repo's own `tokenizer.json`, and
//! per-voice style vectors (`voices/<id>.bin`, 510 × 256 f32 each). The
//! inference mechanics shared with Kitten Nano live in `style_tts2.rs`
//! (ADR 0023).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use alp_media::PcmBuffer;
use alp_voice::{TtsBackend, TtsRequest, VoiceError, VoiceInfo};
use piper_plus_g2p::english::EnglishPhonemizer;

use super::style_tts2::{
    NeuralSession, list_voices, parse_vocab, style_row, synthesize_text, voice_path,
};

/// Kokoro v1.0 emits 24 kHz mono.
pub const SAMPLE_RATE: u32 = 24_000;

/// Catalogue id of the model this engine executes.
pub const MODEL_ID: &str = "kokoro-82m";

/// The fp32 ONNX export under `onnx/` in `onnx-community/Kokoro-82M-v1.0-ONNX`.
const MODEL_FILES: [&str; 3] = ["onnx/model.onnx", "kokoro-v1.0.onnx", "model.onnx"];
/// Character vocabulary: `{ "model": { "vocab": { "<char>": <id> } } }`.
const TOKENIZER_FILE: &str = "tokenizer.json";
/// Per-voice style vectors, one 256-entry f32 row per chunk-length bucket.
const VOICES_SUBDIR: &str = "voices";
/// Friendly fallback when a request names an unknown voice.
const DEFAULT_VOICE: &str = "af_heart";
/// Voice file stem -> (display name, alias words a request might use).
const VOICES: &[(&str, &str, &[&str])] = &[
    ("af_heart", "Heart (US female)", &["heart"]),
    ("af_bella", "Bella (US female)", &["bella"]),
    ("am_michael", "Michael (US male)", &["michael"]),
];

/// A loaded Kokoro engine: session + vocabulary + phonemizer + model dir.
pub struct KokoroEngine {
    /// ONNX session over the shared architecture layer.
    session: NeuralSession,
    phonemizer: EnglishPhonemizer,
    vocab: HashMap<char, i64>,
    model_dir: PathBuf,
}

impl KokoroEngine {
    /// Load the engine from a model directory containing `onnx/model.onnx`
    /// (or `kokoro-v1.0.onnx`), `tokenizer.json` and a `voices/` directory.
    ///
    /// Fails before touching ONNX Runtime when required files are missing, so
    /// the error names the fix instead of a raw ORT load failure.
    pub fn load(model_dir: &Path) -> Result<Self, VoiceError> {
        let model_path = MODEL_FILES
            .iter()
            .map(|name| model_dir.join(name))
            .find(|path| path.is_file())
            .ok_or_else(|| {
                VoiceError::Unavailable(format!(
                    "Kokoro weights not found in {} (expected {}); run \
                     `alp-cli model download kokoro-82m` to install them",
                    model_dir.display(),
                    MODEL_FILES[0]
                ))
            })?;
        let tokenizer_path = model_dir.join(TOKENIZER_FILE);
        if !tokenizer_path.is_file() {
            return Err(VoiceError::Unavailable(format!(
                "Kokoro vocabulary not found: {} is missing",
                tokenizer_path.display()
            )));
        }
        let voices_dir = model_dir.join(VOICES_SUBDIR);
        if !voices_dir.is_dir() {
            return Err(VoiceError::Unavailable(format!(
                "Kokoro voice styles not found: {} is missing",
                voices_dir.display()
            )));
        }

        let vocab = parse_vocab(&tokenizer_path)
            .map_err(|e| VoiceError::Backend(format!("tokenizer.json unreadable: {e}")))?;

        // Fail early on a model whose tensor names do not match the v1.0 export.
        let session = NeuralSession::load(
            &model_path,
            "alp-infer-kokoro",
            &["input_ids", "style", "speed"],
        )?;

        let phonemizer = EnglishPhonemizer::new_bundled()
            .map_err(|e| VoiceError::Backend(format!("G2P init failed: {e}")))?;

        Ok(Self {
            session,
            phonemizer,
            vocab,
            model_dir: model_dir.to_path_buf(),
        })
    }

    /// Style vector for a voice, falling back to the default voice when the
    /// requested one is not installed.
    fn style_for(&self, voice: &str, token_count: usize) -> Result<Vec<f32>, VoiceError> {
        let voices_dir = self.model_dir.join(VOICES_SUBDIR);
        let path = voice_path(&voices_dir, voice, VOICES)
            .or_else(|| voice_path(&voices_dir, DEFAULT_VOICE, VOICES))
            .ok_or_else(|| {
                VoiceError::UnknownVoice(format!(
                    "no style vector installed for voice {voice:?} in {}",
                    voices_dir.display()
                ))
            })?;
        let bin = std::fs::read(&path)
            .map_err(|e| VoiceError::Backend(format!("reading {}: {e}", path.display())))?;
        style_row(&bin, token_count)
    }
}

impl TtsBackend for KokoroEngine {
    fn id(&self) -> &str {
        MODEL_ID
    }

    fn voices(&self) -> Vec<VoiceInfo> {
        list_voices(&self.model_dir.join(VOICES_SUBDIR), VOICES)
    }

    fn synthesize(&self, request: &TtsRequest) -> Result<PcmBuffer, VoiceError> {
        synthesize_text(
            &self.session,
            &self.phonemizer,
            &self.vocab,
            request,
            SAMPLE_RATE,
            "kokoro",
            &|voice, tokens| self.style_for(voice, tokens),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn written(dir: &Path, files: &[(&str, &str)]) -> PathBuf {
        for (name, contents) in files {
            let path = dir.join(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("create dirs");
            }
            fs::write(path, contents).expect("write fixture");
        }
        dir.to_path_buf()
    }

    #[test]
    fn voice_path_resolves_stems_aliases_and_friendly_names() {
        let dir = tempfile::tempdir().expect("tempdir");
        let voices = written(
            dir.path(),
            &[("voices/af_heart.bin", "x"), ("voices/af_bella.bin", "x")],
        )
        .join("voices");

        let exact = voice_path(&voices, "af_heart", VOICES).expect("stem");
        assert!(exact.ends_with("af_heart.bin"));

        let alias = voice_path(&voices, "heart", VOICES).expect("alias");
        assert!(alias.ends_with("af_heart.bin"));

        let display = voice_path(&voices, "Bella (US female)", VOICES).expect("display name");
        assert!(display.ends_with("af_bella.bin"));

        assert!(voice_path(&voices, "zz_unknown", VOICES).is_none());
        let empty = tempfile::tempdir().expect("tempdir");
        assert!(voice_path(&empty.path().join("voices"), "af_heart", VOICES).is_none());
    }

    #[test]
    fn list_voices_reports_installed_bins_with_display_names() {
        let dir = tempfile::tempdir().expect("tempdir");
        let voices = written(
            dir.path(),
            &[("voices/af_heart.bin", "x"), ("voices/am_michael.bin", "x")],
        )
        .join("voices");
        let voices = list_voices(&voices, VOICES);
        assert_eq!(voices.len(), 2);
        let heart = voices.iter().find(|v| v.id == "af_heart").expect("heart");
        assert_eq!(heart.name, "Heart (US female)");
        assert_eq!(heart.language, "en");
        let michael = voices
            .iter()
            .find(|v| v.id == "am_michael")
            .expect("michael");
        assert_eq!(michael.name, "Michael (US male)");
    }

    /// Long text must be reconstructed from sentence chunks (ADR 0024), not
    /// shortened to one oversized token sequence. The paragraph is engineered
    /// to exceed the 320-character budget only after greedy packing, so the
    /// chunker really splits.
    #[test]
    fn long_text_is_reconstructed_from_chunks_when_installed() {
        let model_dir = std::env::var("ALP_KOKORO_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                alp_voice::ModelManagerConfig::default()
                    .storage_dir
                    .join("kokoro-82m")
            });
        if !model_dir.join("onnx/model.onnx").is_file()
            && !model_dir.join("kokoro-v1.0.onnx").is_file()
            && !model_dir.join("model.onnx").is_file()
        {
            eprintln!("skip: Kokoro model not installed");
            return;
        }

        let engine = KokoroEngine::load(&model_dir).expect("load");
        let pcm = engine
            .synthesize(&TtsRequest {
                text: "Adaptive audio tools have grown considerably over the past decade. \
                       Every cutting pass is previewed before it ever touches the timeline. \
                       The undo stack keeps every decision reversible. \
                       Speech synthesis runs inside a separate daemon. \
                       Long lectures and audiobook chapters depend on this."
                    .into(),
                voice_id: "af_heart".into(),
                speed: 1.0,
            })
            .expect("chunked synthesis");
        let seconds = pcm.samples().len() as f64 / f64::from(SAMPLE_RATE);
        eprintln!("kokoro chunked: {seconds:.2}s of audio");
        assert_eq!(pcm.sample_rate(), SAMPLE_RATE);
        assert_eq!(pcm.channels(), 1);
        assert!(
            seconds > 12.0,
            "expected the full text rendered, got {seconds:.2}s"
        );
    }

    /// End-to-end smoke against the real downloaded model. Runs only when the
    /// model is installed (standard XDG location or `ALP_KOKORO_DIR`), so CI
    /// with no model stays green.
    #[test]
    fn real_model_synthesizes_speech_when_installed() {
        let model_dir = std::env::var("ALP_KOKORO_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                alp_voice::ModelManagerConfig::default()
                    .storage_dir
                    .join("kokoro-82m")
            });
        if !model_dir.join("onnx/model.onnx").is_file()
            && !model_dir.join("kokoro-v1.0.onnx").is_file()
            && !model_dir.join("model.onnx").is_file()
        {
            eprintln!(
                "skip: Kokoro model not installed at {}",
                model_dir.display()
            );
            return;
        }

        let engine = KokoroEngine::load(&model_dir).expect("load");
        let started = std::time::Instant::now();
        let pcm = engine
            .synthesize(&TtsRequest {
                text: "Hello from Audio Labs Pro.".into(),
                voice_id: "af_heart".into(),
                speed: 1.0,
            })
            .expect("synthesize");
        let elapsed = started.elapsed();
        let seconds = pcm.samples().len() as f64 / f64::from(SAMPLE_RATE);
        eprintln!(
            "kokoro smoke: {elapsed:?} for {seconds:.2}s audio (RTF {:.3}, 24 kHz)",
            elapsed.as_secs_f64() / seconds
        );

        assert_eq!(pcm.sample_rate(), SAMPLE_RATE);
        assert_eq!(pcm.channels(), 1);
        assert!(
            pcm.samples().len() > SAMPLE_RATE as usize / 2,
            "expected > 0.5 s of speech"
        );
        let peak = pcm
            .samples()
            .iter()
            .copied()
            .fold(0.0f32, |m, s| m.max(s.abs()));
        let rms = (pcm
            .samples()
            .iter()
            .map(|value| (*value as f64) * (*value as f64))
            .sum::<f64>()
            / pcm.samples().len() as f64)
            .sqrt();
        assert!(peak > 0.05, "non-silent peak expected, got {peak}");
        assert!(
            rms > 0.005 && rms < 0.5,
            "speech-like RMS expected, got {rms} (spike measured ~0.066)"
        );
    }
}
