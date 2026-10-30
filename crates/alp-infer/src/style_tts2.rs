//! Shared mechanics for `style_text_to_speech_2`-family engines (ADR 0023).
//!
//! Kokoro-82M and Kitten Nano are the same architecture family: a character
//! vocabulary over IPA, 0 (`$`) padding at both ends, a 256-lane f32 style
//! vector per voice, and tensors named `input_ids` / `style` / `speed`
//! producing a mono waveform. This module holds exactly that shared
//! machinery; `kokoro.rs` and `kitten.rs` keep only model facts (paths,
//! sample rate, voice tables, catalogue id).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use alp_media::PcmBuffer;
use alp_voice::text::TextNormalizer;
use alp_voice::{TtsRequest, VoiceError, VoiceInfo, VoiceKind};
use ort::environment::Environment;
use ort::session::Session;
use ort::value::Tensor;
use piper_plus_g2p::Phonemizer;
use piper_plus_g2p::english::EnglishPhonemizer;

/// The process-wide ONNX Runtime environment.
///
/// ONNX Runtime permits only one live `OrtEnv` per process, so every engine
/// session shares this single instance instead of building its own (an early
/// per-engine environment survived dual-engine tests only by drop timing).
static RUNTIME_ENV: OnceLock<Environment> = OnceLock::new();

/// Borrow (initializing once) the shared runtime environment.
pub(crate) fn runtime_environment() -> &'static Environment {
    RUNTIME_ENV.get_or_init(|| {
        ort::init()
            .with_name("alp-infer")
            .build()
            .expect("ONNX Runtime environment init")
    })
}

/// Bytes per style row: 256 f32 lanes.
const STYLE_ROW_BYTES: usize = 256 * std::mem::size_of::<f32>();

/// Parse the character vocabulary out of a `tokenizer.json`.
///
/// Both engines share the layout: the canonical `{ "model": { "vocab": {
/// "<char>": <id> } } }` (Kokoro's export and Kitten's HF `tokenizers` file
/// alike); a top-level `vocab` is accepted as a fallback.
pub(super) fn parse_vocab(path: &Path) -> Result<HashMap<char, i64>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let entries = json
        .pointer("/model/vocab")
        .or_else(|| json.get("vocab"))
        .and_then(|v| v.as_object())
        .ok_or("vocab object not found")?;

    let mut vocab = HashMap::with_capacity(entries.len());
    for (token, id) in entries {
        let id = id
            .as_i64()
            .ok_or_else(|| format!("non-numeric id for {token:?}"))?;
        let mut chars = token.chars();
        if let (Some(character), None) = (chars.next(), chars.next()) {
            vocab.insert(character, id);
        }
    }
    if vocab.is_empty() {
        return Err("vocabulary is empty".into());
    }
    Ok(vocab)
}

/// Map IPA characters to token ids (0-padded both ends; `$` is id 0 in both
/// vocabularies). Unknown characters are collected so callers can report
/// them instead of failing the request.
pub(super) fn token_ids(vocab: &HashMap<char, i64>, ipa: &str) -> (Vec<i64>, String) {
    let mut ids = vec![0_i64];
    let mut skipped = String::new();
    for character in ipa.chars() {
        match vocab.get(&character) {
            Some(id) => ids.push(*id),
            None => skipped.push(character),
        }
    }
    ids.push(0);
    (ids, skipped)
}

/// Select one 256-lane f32 style row: `min(token_count, rows - 1)`.
///
/// Kokoro ships length-bucketed rows (510 per voice); Kitten ships exactly
/// one row per voice, so the clamp resolves to row 0 there.
pub(super) fn style_row(bin: &[u8], token_count: usize) -> Result<Vec<f32>, VoiceError> {
    let rows = bin.len() / STYLE_ROW_BYTES;
    if rows == 0 {
        return Err(VoiceError::Backend(format!(
            "style file holds no {}-byte rows ({} bytes total)",
            STYLE_ROW_BYTES,
            bin.len()
        )));
    }
    let index = token_count.min(rows - 1);
    let row = &bin[index * STYLE_ROW_BYTES..(index + 1) * STYLE_ROW_BYTES];
    Ok(row
        .as_chunks::<4>()
        .0
        .iter()
        .map(|bytes| f32::from_le_bytes(*bytes))
        .collect())
}

/// Sorted list of style-bin file names in a voices directory.
pub(super) fn style_bins(voices_dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(voices_dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "bin"))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    names.sort();
    names
}

/// Lowercase letters and digits only, so catalogue display names resolve
/// against file stems: `Expr Voice 2 (F)` -> `exprvoice2f` matches
/// `expr-voice-2-f`.
fn canonical_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

/// Resolve a requested voice id or friendly name to a style bin path.
///
/// Matching order: exact file stem, known catalogue stem or alias word, a
/// tail/contains match, then canonical equality so `Expr Voice 2 (F)`
/// resolves to `expr-voice-2-f.bin`. A parenthesised qualifier is stripped
/// before the prefix passes, so `Bella (US female)` still matches by stem
/// tail on Kokoro.
pub(super) fn voice_path(
    voices_dir: &Path,
    voice_id: &str,
    table: &[(&str, &str, &[&str])],
) -> Option<PathBuf> {
    let files = style_bins(voices_dir);
    if files.is_empty() {
        return None;
    }
    let base = voice_id.trim().to_lowercase();
    let sans_ext = base
        .rsplit_once('.')
        .map_or(base.as_str(), |(stem, _)| stem);
    // Drop a parenthesised qualifier: "bella (us female)" -> "bella".
    let wanted = sans_ext.split('(').next().unwrap_or(sans_ext).trim();
    if wanted.is_empty() {
        return None;
    }

    let stems: Vec<String> = files
        .iter()
        .map(|file| file.trim_end_matches(".bin").to_lowercase())
        .collect();

    if let Some(index) = stems.iter().position(|stem| *stem == wanted) {
        return Some(voices_dir.join(&files[index]));
    }
    for (stem, _, aliases) in table {
        if stem == &wanted || aliases.contains(&wanted) {
            return Some(voices_dir.join(format!("{stem}.bin")));
        }
    }
    for (index, stem) in stems.iter().enumerate() {
        if stem.ends_with(wanted) || wanted.ends_with(stem.as_str()) {
            return Some(voices_dir.join(&files[index]));
        }
    }
    for (index, stem) in stems.iter().enumerate() {
        if wanted.contains(stem.as_str()) || stem.contains(wanted) {
            return Some(voices_dir.join(&files[index]));
        }
    }
    // Canonical pass on the *unstripped* name: "Expr Voice 4 (M)" and
    // "expr-voice-4-m" both reduce to "exprvoice4m". The parenthesised
    // qualifier is part of the identity here, so it must not be dropped
    // before comparison.
    let canonical_wanted = canonical_name(sans_ext);
    if canonical_wanted.len() >= 3 {
        for (index, stem) in stems.iter().enumerate() {
            if canonical_name(stem) == canonical_wanted {
                return Some(voices_dir.join(&files[index]));
            }
        }
    }
    None
}

/// Discover installed voices for the `voices()` listing, using the model's
/// catalogue table for display names.
pub(super) fn list_voices(voices_dir: &Path, table: &[(&str, &str, &[&str])]) -> Vec<VoiceInfo> {
    style_bins(voices_dir)
        .into_iter()
        .map(|file| {
            let stem = file.trim_end_matches(".bin").to_owned();
            let name = table
                .iter()
                .find(|(known, _, _)| *known == stem.as_str())
                .map(|(_, display, _)| (*display).to_owned())
                .unwrap_or_else(|| stem.clone());
            VoiceInfo {
                id: stem,
                name,
                language: "en".into(),
                kind: VoiceKind::Builtin,
            }
        })
        .collect()
}

/// Text to IPA through the MIT G2P front-end (ADR 0010: no espeak-ng).
pub(super) fn phonemize(phonemizer: &EnglishPhonemizer, text: &str) -> Result<String, VoiceError> {
    let (tokens, _prosody) = phonemizer
        .phonemize_with_prosody(text)
        .map_err(|e| VoiceError::Backend(format!("phonemization failed: {e}")))?;
    Ok(tokens.concat())
}

/// A loaded `style_text_to_speech_2` ONNX graph plus its runtime session.
///
/// The graph hangs off the shared [`runtime_environment`]; `run` needs
/// `&mut Session`, so the session sits behind a mutex; the daemon serves one
/// request at a time on its accept loop.
pub(super) struct NeuralSession {
    session: Mutex<Session>,
    input_names: Vec<String>,
}

impl NeuralSession {
    /// Open a session over `model_path`, refusing graphs whose tensor names
    /// do not cover every name in `required`.
    pub(super) fn load(
        model_path: &Path,
        _environment_name: &str,
        required: &[&str],
    ) -> Result<Self, VoiceError> {
        let started = std::time::Instant::now();
        let mut builder = Session::builder(runtime_environment()).map_err(|e| {
            VoiceError::Backend(format!("ONNX Runtime session builder failed: {e}"))
        })?;
        let session = builder
            .commit_from_file(model_path)
            .map_err(|e| VoiceError::Backend(format!("loading {}: {e}", model_path.display())))?;

        let input_names: Vec<String> = session
            .inputs()
            .iter()
            .map(|input| input.name().to_owned())
            .collect();
        for expected in required {
            if !input_names.iter().any(|name| name == expected) {
                return Err(VoiceError::Backend(format!(
                    "{} is not a compatible style_text_to_speech_2 model: missing input \
                     `{expected}` (found {input_names:?})",
                    model_path.display()
                )));
            }
        }

        eprintln!(
            "neural engine: loaded {} ({:.1?})",
            model_path.display(),
            started.elapsed()
        );

        Ok(Self {
            session: Mutex::new(session),
            input_names,
        })
    }

    /// Whether the graph accepts an optional input such as `speed`.
    pub(super) fn has_input(&self, name: &str) -> bool {
        self.input_names.iter().any(|n| n == name)
    }

    /// Run one inference: phoneme ids + 256-lane style (+ speed when the
    /// export provides it) and read back the mono waveform.
    pub(super) fn synthesize_tokens(
        &self,
        ids: &[i64],
        style: &[f32],
        speed: f32,
    ) -> Result<Vec<f32>, VoiceError> {
        let tokens = ids.len() as i64;
        let input_ids = Tensor::from_array((vec![1, tokens], ids.to_vec()))
            .map_err(|e| VoiceError::Backend(format!("token tensor: {e}")))?;
        let style_tensor = Tensor::from_array((vec![1, 256], style.to_vec()))
            .map_err(|e| VoiceError::Backend(format!("style tensor: {e}")))?;
        let speed_tensor = Tensor::from_array((vec![1], vec![speed]))
            .map_err(|e| VoiceError::Backend(format!("speed tensor: {e}")))?;

        let mut session = self
            .session
            .lock()
            .map_err(|_| VoiceError::Backend("engine session lock poisoned".into()))?;
        let outputs = if self.has_input("speed") {
            session
                .run(ort::inputs![
                    "input_ids" => input_ids,
                    "style" => style_tensor,
                    "speed" => speed_tensor,
                ])
                .map_err(|e| VoiceError::Backend(format!("inference failed: {e}")))?
        } else {
            session
                .run(ort::inputs!["input_ids" => input_ids, "style" => style_tensor])
                .map_err(|e| VoiceError::Backend(format!("inference failed: {e}")))?
        };
        let (_shape, samples) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| VoiceError::Backend(format!("reading audio output: {e}")))?;
        let samples: Vec<f32> = samples.to_vec();
        if samples.is_empty() {
            return Err(VoiceError::Backend("the model produced no samples".into()));
        }
        Ok(samples)
    }
}

/// Silence gap inserted between concatenated sentence chunks when long text
/// is synthesized in pieces.
pub(super) const CHUNK_GAP_SECS: f32 = 0.15;

/// Character budget for one synthesis chunk (ADR 0024). English IPA runs
/// roughly 1.5 characters per phoneme, so a 320-character piece lands well
/// inside the Kokoro style-row buckets and keeps per-request latency sane.
const CHUNK_CHAR_BUDGET: usize = 320;

/// The per-model style resolver handed to the shared synthesis flow.
type StyleResolver<'a> = &'a dyn Fn(&str, usize) -> Result<Vec<f32>, VoiceError>;

/// The full synthesis flow shared by every `style_text_to_speech_2` engine:
/// normalize, split long text into sentence chunks, phonemize, map to token
/// ids, select the style vector, run the graph, and concatenate with short
/// gaps (ADR 0024).
///
/// `style_for` is the model-specific vector resolver; `log_prefix` names the
/// engine in stderr diagnostics.
pub(super) fn synthesize_text(
    session: &NeuralSession,
    phonemizer: &EnglishPhonemizer,
    vocab: &HashMap<char, i64>,
    request: &TtsRequest,
    sample_rate: u32,
    log_prefix: &str,
    style_for: StyleResolver<'_>,
) -> Result<PcmBuffer, VoiceError> {
    let normalized = TextNormalizer::normalize(&request.text);
    if normalized.trim().is_empty() {
        return PcmBuffer::new(sample_rate, 1, vec![0.0; 100])
            .map_err(|e| VoiceError::Backend(e.to_string()));
    }

    let speed = request.speed.clamp(0.5, 2.0);
    let mut waveforms: Vec<Vec<f32>> = Vec::new();
    let mut skipped = String::new();
    for chunk in TextNormalizer::sentence_chunks(&normalized, CHUNK_CHAR_BUDGET) {
        let ipa = phonemize(phonemizer, &chunk)?;
        let (ids, chunk_skipped) = token_ids(vocab, &ipa);
        skipped.push_str(&chunk_skipped);
        if ids.len() <= 2 {
            continue; // an inter-phrase chunk with no speakable content
        }
        let style = style_for(&request.voice_id, ids.len())?;
        waveforms.push(session.synthesize_tokens(&ids, &style, speed)?);
    }
    if waveforms.is_empty() {
        return Err(VoiceError::Backend(format!(
            "text produced no speakable phonemes (skipped: {skipped:?})"
        )));
    }
    if !skipped.is_empty() {
        eprintln!("{log_prefix}: skipped unwritable characters {skipped:?}");
    }
    if waveforms.len() > 1 {
        eprintln!(
            "{log_prefix}: reconstructed {} chunks (ADR 0024)",
            waveforms.len()
        );
    }
    let samples = concatenate_chunks(sample_rate, &waveforms);
    PcmBuffer::new(sample_rate, 1, samples).map_err(|e| VoiceError::Backend(e.to_string()))
}

/// Concatenate per-chunk waveforms with a short silence between them.
/// Empty chunks are ignored and never produce gaps.
pub(super) fn concatenate_chunks(sample_rate: u32, chunks: &[Vec<f32>]) -> Vec<f32> {
    let gap = (sample_rate as f32 * CHUNK_GAP_SECS) as usize;
    let active: Vec<&Vec<f32>> = chunks.iter().filter(|chunk| !chunk.is_empty()).collect();
    let total: usize = active.iter().map(|chunk| chunk.len()).sum::<usize>()
        + active.len().saturating_sub(1) * gap;
    let mut out = Vec::with_capacity(total);
    for (index, chunk) in active.iter().enumerate() {
        if index > 0 {
            out.extend(std::iter::repeat_n(0.0_f32, gap));
        }
        out.extend_from_slice(chunk);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Both engines name their vocabulary file `tokenizer.json`.
    const TOKENIZER_FILE: &str = "tokenizer.json";

    /// Fixture vocabulary: pad(0), ^, $, h, e, l, o, space.
    const MIN_TOKENIZER: &str =
        r#"{"model":{"vocab":{"^":1,"$":2,"h":7,"e":8,"l":17,"o":9," ":25}}}"#;

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
    fn parse_vocab_reads_the_model_vocab_and_maps_single_chars() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = written(dir.path(), &[(TOKENIZER_FILE, MIN_TOKENIZER)]).join(TOKENIZER_FILE);
        let vocab = parse_vocab(&path).expect("parse");
        assert_eq!(vocab[&'^'], 1);
        assert_eq!(vocab[&'h'], 7);
        assert_eq!(vocab[&' '], 25);
        assert_eq!(vocab.len(), 7);
    }

    #[test]
    fn parse_vocab_accepts_hf_tokenizer_layouts_like_kittens() {
        // Kitten's export is an HF `tokenizers` file whose vocabulary also
        // lives at model.vocab; the same parser serves both engines.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = written(
            dir.path(),
            &[(
                TOKENIZER_FILE,
                r#"{"version":"1.0","model":{"type":"WordLevel","vocab":{"$":0," ":16}}}"#,
            )],
        )
        .join(TOKENIZER_FILE);
        let vocab = parse_vocab(&path).expect("parse");
        assert_eq!(vocab[&'$'], 0);
        assert_eq!(vocab[&' '], 16);
    }

    #[test]
    fn parse_vocab_rejects_empty_or_absent_vocab() {
        let dir = tempfile::tempdir().expect("tempdir");
        let empty =
            written(dir.path(), &[("tok.json", r#"{"model":{"vocab":{}}}"#)]).join("tok.json");
        assert!(parse_vocab(&empty).is_err());
        assert!(parse_vocab(&dir.path().join("absent.json")).is_err());
    }

    #[test]
    fn token_ids_pad_both_ends_and_skip_unknown() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = written(dir.path(), &[(TOKENIZER_FILE, MIN_TOKENIZER)]).join(TOKENIZER_FILE);
        let vocab = parse_vocab(&path).expect("parse");

        let (ids, skipped) = token_ids(&vocab, "hello ");
        assert_eq!(ids, vec![0, 7, 8, 17, 17, 9, 25, 0]);
        assert!(skipped.is_empty());

        let (_, skipped) = token_ids(&vocab, "θʒ");
        assert_eq!(skipped, "θʒ");
    }

    #[test]
    fn style_row_selects_by_token_count_and_clamps_to_last_row() {
        // Two rows: row 0 all 1.0, row 1 all 2.0 (little-endian).
        let mut bin = Vec::new();
        for value in [1.0f32, 2.0f32] {
            bin.extend(std::iter::repeat_n(value.to_le_bytes(), 256).flatten());
        }

        let first = style_row(&bin, 0).expect("row 0");
        assert_eq!(first.len(), 256);
        assert!(first.iter().all(|value| *value == 1.0));

        let second = style_row(&bin, 300).expect("clamped row");
        assert!(second.iter().all(|value| *value == 2.0));

        assert!(style_row(&[0, 0, 0], 5).is_err(), "incomplete rows fail");

        let empty: [u8; 0] = [];
        assert!(style_row(&empty, 1).is_err());
    }

    #[test]
    fn single_row_style_files_always_yield_row_zero() {
        // Kitten's voices are exactly one 256-lane row; any token count
        // resolves to it.
        let mut bin = Vec::new();
        bin.extend(std::iter::repeat_n(3.5f32.to_le_bytes(), 256).flatten());
        let row = style_row(&bin, 999).expect("row 0");
        assert_eq!(row.len(), 256);
        assert!(row.iter().all(|value| *value == 3.5));
    }

    #[test]
    fn canonical_voice_pass_matches_manifest_display_names() {
        let dir = tempfile::tempdir().expect("tempdir");
        let table: &[(&str, &str, &[&str])] = &[("expr-voice-2-f", "Expr Voice 2 (F)", &[])];
        let voices = written(dir.path(), &[("voices/expr-voice-2-f.bin", "x")]).join("voices");

        let resolved = voice_path(&voices, "Expr Voice 2 (F)", table).expect("canonical");
        assert!(resolved.ends_with("expr-voice-2-f.bin"));

        // The pass must not invent matches for unrelated short input.
        assert!(voice_path(&voices, "ab (x)", table).is_none());
    }

    #[test]
    fn concatenated_chunks_carry_silence_gaps_between_full_chunks() {
        let chunks = vec![vec![1.0f32, 1.0], vec![2.0f32], vec![]];
        let joined = concatenate_chunks(10_000, &chunks);
        // The trailing empty chunk is ignored: 10 kHz * 0.15 s = 1500 silence
        // samples between the two non-empty chunks only.
        assert_eq!(joined.len(), 2 + 1500 + 1);
        assert!(joined[2..2 + 1500].iter().all(|s| *s == 0.0));
        assert_eq!(joined[0], 1.0);
        assert_eq!(joined[joined.len() - 1], 2.0);

        // A lone chunk concatenates without any gap.
        let solo = concatenate_chunks(10_000, &[vec![0.5f32; 10], vec![], vec![]]);
        assert_eq!(solo.len(), 10);
        assert!(solo.iter().all(|s| *s == 0.5));
    }
}
