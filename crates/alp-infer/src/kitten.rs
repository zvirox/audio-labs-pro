//! Kitten Nano neural TTS engine executed through ONNX Runtime (ADR 0023).
//!
//! The Apache-2.0 `onnx-community/kitten-tts-nano-0.1-ONNX` export: a ~24 MB
//! quantized `style_text_to_speech_2` graph running at 24 kHz mono, one
//! 256-lane f32 style vector per voice (`voices/expr-voice-*-f|m.bin`), and
//! an HF `tokenizers` vocabulary whose `model.vocab` maps the same IPA
//! character set the Kokoro engine feeds. Phonemization stays on the MIT
//! `piper-plus-g2p` path (ADR 0010); no espeak-ng.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use alp_media::PcmBuffer;
use alp_voice::{TtsBackend, TtsRequest, VoiceError, VoiceInfo};
use piper_plus_g2p::english::EnglishPhonemizer;

use super::style_tts2::{
    NeuralSession, list_voices, parse_vocab, style_row, synthesize_text, voice_path,
};

/// Kitten Nano emits 24 kHz mono (per the KittenML model card).
pub const SAMPLE_RATE: u32 = 24_000;

/// Catalogue id of the model this engine executes.
pub const MODEL_ID: &str = "kitten-nano";

/// The quantized export pinned by the manifest; plain `model.onnx` accepted
/// as a manual-install fallback.
const MODEL_FILES: [&str; 4] = [
    "onnx/model_quantized.onnx",
    "onnx/model.onnx",
    "model_quantized.onnx",
    "model.onnx",
];
/// HF `tokenizers` file; the vocabulary lives at `model.vocab`.
const TOKENIZER_FILE: &str = "tokenizer.json";
/// One 256-entry f32 style row per voice.
const VOICES_SUBDIR: &str = "voices";
/// Friendly fallback when a request names an unknown voice.
const DEFAULT_VOICE: &str = "expr-voice-2-f";
/// Voice file stem -> (display name, alias words a request might use).
const VOICES: &[(&str, &str, &[&str])] = &[
    ("expr-voice-2-f", "Expr Voice 2 (F)", &[]),
    ("expr-voice-2-m", "Expr Voice 2 (M)", &[]),
    ("expr-voice-3-f", "Expr Voice 3 (F)", &[]),
    ("expr-voice-3-m", "Expr Voice 3 (M)", &[]),
    ("expr-voice-4-f", "Expr Voice 4 (F)", &[]),
    ("expr-voice-4-m", "Expr Voice 4 (M)", &[]),
    ("expr-voice-5-f", "Expr Voice 5 (F)", &[]),
    ("expr-voice-5-m", "Expr Voice 5 (M)", &[]),
];

/// A loaded Kitten Nano engine: session + vocabulary + phonemizer + model dir.
pub struct KittenEngine {
    /// ONNX session over the shared architecture layer.
    session: NeuralSession,
    phonemizer: EnglishPhonemizer,
    vocab: HashMap<char, i64>,
    model_dir: PathBuf,
}

impl KittenEngine {
    /// Load the engine from a model directory containing the quantized
    /// weights, `tokenizer.json` and a `voices/` directory.
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
                    "Kitten Nano weights not found in {} (expected {}); run \
                     `alp-cli model download kitten-nano` to install them",
                    model_dir.display(),
                    MODEL_FILES[0]
                ))
            })?;
        let tokenizer_path = model_dir.join(TOKENIZER_FILE);
        if !tokenizer_path.is_file() {
            return Err(VoiceError::Unavailable(format!(
                "Kitten Nano vocabulary not found: {} is missing",
                tokenizer_path.display()
            )));
        }
        let voices_dir = model_dir.join(VOICES_SUBDIR);
        if !voices_dir.is_dir() {
            return Err(VoiceError::Unavailable(format!(
                "Kitten Nano voice styles not found: {} is missing",
                voices_dir.display()
            )));
        }

        let vocab = parse_vocab(&tokenizer_path)
            .map_err(|e| VoiceError::Backend(format!("tokenizer.json unreadable: {e}")))?;

        // `speed` is optional here: the quantized export is validated on the
        // two inputs the architecture family always has, and the shared
        // session binds `speed` only when the graph provides it.
        let session =
            NeuralSession::load(&model_path, "alp-infer-kitten", &["input_ids", "style"])?;

        let phonemizer = EnglishPhonemizer::new_bundled()
            .map_err(|e| VoiceError::Backend(format!("G2P init failed: {e}")))?;

        Ok(Self {
            session,
            phonemizer,
            vocab,
            model_dir: model_dir.to_path_buf(),
        })
    }

    /// Style vector for a voice (Kitten ships exactly one row per voice),
    /// falling back to the default voice when the requested one is missing.
    #[allow(dead_code)]
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

impl TtsBackend for KittenEngine {
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
            "kitten",
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
    fn voice_resolution_covers_manifest_display_names_and_stems() {
        let dir = tempfile::tempdir().expect("tempdir");
        let names: Vec<String> = VOICES
            .iter()
            .map(|(stem, _, _)| format!("voices/{stem}.bin"))
            .collect();
        let files: Vec<(&str, &str)> = names.iter().map(|f| (f.as_str(), "x")).collect();
        let voices = written(dir.path(), &files).join("voices");

        // Manifest-style display name resolves through the canonical pass.
        let display = voice_path(&voices, "Expr Voice 4 (M)", VOICES).expect("display");
        assert!(display.ends_with("expr-voice-4-m.bin"));

        // Bare stems resolve directly.
        let stem = voice_path(&voices, "expr-voice-3-f", VOICES).expect("stem");
        assert!(stem.ends_with("expr-voice-3-f.bin"));

        // Unknown voices miss; the engine falls back to its default.
        assert!(voice_path(&voices, "aura", VOICES).is_none());
    }

    #[test]
    fn list_voices_reports_all_installed_voices_with_display_names() {
        let dir = tempfile::tempdir().expect("tempdir");
        let voices = written(
            dir.path(),
            &[
                ("voices/expr-voice-2-f.bin", "x"),
                ("voices/expr-voice-2-m.bin", "x"),
            ],
        )
        .join("voices");
        let voices = list_voices(&voices, VOICES);
        assert_eq!(voices.len(), 2);
        let female = voices
            .iter()
            .find(|v| v.id == "expr-voice-2-f")
            .expect("2-f");
        assert_eq!(female.name, "Expr Voice 2 (F)");
        let male = voices
            .iter()
            .find(|v| v.id == "expr-voice-2-m")
            .expect("2-m");
        assert_eq!(male.name, "Expr Voice 2 (M)");
    }

    #[test]
    fn load_fails_honestly_when_weights_are_absent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let error = KittenEngine::load(&dir.path().join("kitten-nano"))
            .err()
            .expect("load must fail");
        match error {
            VoiceError::Unavailable(message) => {
                assert!(message.contains("kitten-nano"), "{message}");
                assert!(message.contains("alp-cli model download"), "{message}");
            }
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    /// Long text must be reconstructed from sentence chunks (ADR 0024).
    /// Kitten's tiny graph makes this the cheapest full-path check of the
    /// shared chunked flow.
    #[test]
    fn long_text_is_reconstructed_from_chunks_when_installed() {
        let model_dir = std::env::var("ALP_KITTEN_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                alp_voice::ModelManagerConfig::default()
                    .storage_dir
                    .join("kitten-nano")
            });
        if !model_dir.join("onnx/model_quantized.onnx").is_file()
            && !model_dir.join("onnx/model.onnx").is_file()
            && !model_dir.join("model_quantized.onnx").is_file()
            && !model_dir.join("model.onnx").is_file()
        {
            eprintln!("skip: Kitten Nano model not installed");
            return;
        }

        let engine = KittenEngine::load(&model_dir).expect("load");
        let pcm = engine
            .synthesize(&TtsRequest {
                text: "Adaptive audio tools have grown considerably over the past decade. \
                       Every cutting pass is previewed before it ever touches the timeline. \
                       The undo stack keeps every decision reversible. \
                       Speech synthesis runs inside a separate daemon. \
                       Long lectures and audiobook chapters depend on this."
                    .into(),
                voice_id: "Expr Voice 3 (F)".into(),
                speed: 1.0,
            })
            .expect("chunked synthesis");
        let seconds = pcm.samples().len() as f64 / f64::from(SAMPLE_RATE);
        eprintln!("kitten chunked: {seconds:.2}s of audio");
        assert_eq!(pcm.sample_rate(), SAMPLE_RATE);
        assert_eq!(pcm.channels(), 1);
        assert!(
            seconds > 12.0,
            "expected the full text rendered, got {seconds:.2}s"
        );
    }

    /// End-to-end smoke against the real downloaded model. Runs only when the
    /// model is installed (standard XDG location or `ALP_KITTEN_DIR`), so CI
    /// with no model stays green.
    #[test]
    fn real_model_synthesizes_speech_when_installed() {
        let model_dir = std::env::var("ALP_KITTEN_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                alp_voice::ModelManagerConfig::default()
                    .storage_dir
                    .join("kitten-nano")
            });
        if !model_dir.join("onnx/model_quantized.onnx").is_file()
            && !model_dir.join("onnx/model.onnx").is_file()
            && !model_dir.join("model_quantized.onnx").is_file()
            && !model_dir.join("model.onnx").is_file()
        {
            eprintln!(
                "skip: Kitten Nano model not installed at {}",
                model_dir.display()
            );
            return;
        }

        let engine = KittenEngine::load(&model_dir).expect("load");
        let started = std::time::Instant::now();
        let pcm = engine
            .synthesize(&TtsRequest {
                text: "This tiny model speaks without a GPU.".into(),
                voice_id: "expr-voice-2-f".into(),
                speed: 1.0,
            })
            .expect("synthesize");
        let elapsed = started.elapsed();
        let seconds = pcm.samples().len() as f64 / f64::from(SAMPLE_RATE);
        eprintln!(
            "kitten smoke: {elapsed:?} for {seconds:.2}s audio (RTF {:.3}, 24 kHz)",
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
            "speech-like RMS expected, got {rms}"
        );
    }
}
