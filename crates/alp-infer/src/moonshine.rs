//! Moonshine Tiny neural speech-to-text engine executed through ONNX Runtime
//! (ADR 0025).
//!
//! Runs the MIT-licensed `onnx-community/moonshine-tiny-ONNX` export: a
//! 16 kHz audio encoder (`input_values` -> 288-wide hidden states) plus a
//! merged decoder graph with a KV-cache branch (`use_cache_branch` selects
//! between full-sequence attention and cache-apended attention). Decoding is
//! greedy: prefill on the decoder start token, then one token per step until
//! `</s>` (id 2). The export's cache branch emits constant-empty
//! `present.*.encoder.*` tensors, so the cross-attention KV are frozen at
//! the prefill values and only the self-attention (decoder) cache rolls
//! forward. Text comes back through the Llama-style BPE in `tokenizer.json`
//! (specials `<s>`=1/`</s>`=2, byte-fallback `<0xNN>` tokens, `\u{2581}` as
//! an encoded space). There are no timestamps; a transcription is one
//! segment spanning the whole buffer, reported in the caller's frame space.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use alp_media::{PcmBuffer, resample};
use alp_voice::{SttBackend, TranscriptSegment, VoiceError};
use ort::environment::Environment;
use ort::session::{Session, SessionInputValue};
use ort::value::Tensor;
use serde_json::Value as Json;

/// Moonshine expects 16 kHz mono speech.
pub const SAMPLE_RATE: u32 = 16_000;

/// Catalogue id of the model this engine executes.
pub const MODEL_ID: &str = "moonshine-tiny";

/// The two ONNX graphs, both under `onnx/` per the manifest.
/// The two ONNX graphs, both under `onnx/` per the manifest.
pub(crate) const ENCODER_FILE: &str = "onnx/encoder_model.onnx";
pub(crate) const DECODER_FILE: &str = "onnx/decoder_model_merged.onnx";
const TOKENIZER_FILE: &str = "tokenizer.json";
const CONFIG_FILE: &str = "config.json";

/// One named KV tensor slot (`past_key_values.*.{decoder,encoder}.{key,value}`).
type KvSlot = (String, Tensor<f32>);

/// One decoder step: logits shape, logits payload, and the new cache.
type StepOutput = (Vec<i64>, Vec<f32>, Vec<KvSlot>);

/// Llama-style BPE map: `\u{2581}` (LOWER ONE EIGHTH BLOCK) encodes a space.
const SPACE_MARK: char = '\u{2581}';

/// Encoder hidden width and the attention geometry, from `config.json`
/// (hidden 288, 8 heads, head_dim 36, 6+6 layers): 24 KV slots in total.
const KV_LAYERS: usize = 6;
const KV_HEADS: i64 = 8;
const KV_HEAD_DIM: i64 = 36;

/// Hard cap on greedy steps, well under `max_position_embeddings` (512).
const MAX_DECODE_STEPS: usize = 448;

/// Longest accepted input, in seconds; longer captures need trimming first
/// (the tiny model is tuned for short utterances).
const MAX_INPUT_SECS: u64 = 300;

/// The process-wide ONNX Runtime environment: one per process, shared with
/// the TTS engines (see `style_tts2::runtime_environment`).
fn runtime_environment() -> &'static Environment {
    crate::style_tts2::runtime_environment()
}

/// The 24 KV-cache slot names in a stable order (past and present share it).
fn kv_names() -> Vec<String> {
    let mut names = Vec::new();
    for layer in 0..KV_LAYERS {
        for part in [
            "decoder.key",
            "decoder.value",
            "encoder.key",
            "encoder.value",
        ] {
            names.push(format!("past_key_values.{layer}.{part}"));
        }
    }
    names
}

/// Empty past tensors shaped `[1, 8, 0, 36]` as the prefill expects.
fn empty_kv_cache() -> Result<Vec<KvSlot>, VoiceError> {
    kv_names()
        .into_iter()
        .map(|name| {
            let tensor = Tensor::from_array((vec![1, KV_HEADS, 0, KV_HEAD_DIM], Vec::<f32>::new()))
                .map_err(|e| VoiceError::Backend(format!("cache tensor: {e}")))?;
            Ok((name, tensor))
        })
        .collect()
}

/// Greedy argmax over one vocabulary-sized logits row.
fn argmax(row: &[f32]) -> i64 {
    row.iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map_or(0, |(index, _)| index as i64)
}

/// Read `tokenizer.json` into an id-to-token table (reverse of
/// `model.vocab`; all specials live in the same vocabulary).
fn parse_tokenizer(path: &Path) -> Result<HashMap<i64, String>, VoiceError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| VoiceError::Backend(format!("reading {}: {e}", path.display())))?;
    let json: Json = serde_json::from_str(&text)
        .map_err(|e| VoiceError::Backend(format!("parsing {}: {e}", path.display())))?;
    let vocab = json
        .pointer("/model/vocab")
        .and_then(Json::as_object)
        .ok_or_else(|| {
            VoiceError::Backend(format!(
                "{}: model.vocab not found (Moonshine's BPE layout)",
                path.display()
            ))
        })?;
    let mut table = HashMap::with_capacity(vocab.len());
    for (token, id) in vocab {
        let id = id.as_i64().ok_or_else(|| {
            VoiceError::Backend(format!("{}: non-numeric id for {token:?}", path.display()))
        })?;
        table.insert(id, token.clone());
    }
    if table.is_empty() {
        return Err(VoiceError::Backend(format!(
            "{}: vocabulary is empty",
            path.display()
        )));
    }
    Ok(table)
}

/// Read `(decoder_start_token_id, eos_token_id)` from `config.json`.
fn parse_special_ids(path: &Path) -> Result<(i64, i64), VoiceError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| VoiceError::Backend(format!("reading {}: {e}", path.display())))?;
    let json: Json = serde_json::from_str(&text)
        .map_err(|e| VoiceError::Backend(format!("parsing {}: {e}", path.display())))?;
    let start = json
        .get("decoder_start_token_id")
        .and_then(Json::as_i64)
        .ok_or_else(|| {
            VoiceError::Backend(format!(
                "{}: decoder_start_token_id missing",
                path.display()
            ))
        })?;
    let eos = json
        .get("eos_token_id")
        .and_then(Json::as_i64)
        .ok_or_else(|| VoiceError::Backend(format!("{}: eos_token_id missing", path.display())))?;
    Ok((start, eos))
}

/// Llama-style BPE decode: skip specials, accumulate `<0xNN>` byte-fallback
/// runs into UTF-8, map the space mark back to a plain space.
fn decode_ids(table: &HashMap<i64, String>, ids: &[i64]) -> String {
    let mut out = String::new();
    let mut bytes: Vec<u8> = Vec::new();
    for id in ids {
        let token = table.get(id).map(String::as_str).unwrap_or("");
        if token.starts_with("<0x")
            && token.ends_with('>')
            && token.len() == 6
            && let Ok(byte) = u8::from_str_radix(&token[3..5], 16)
        {
            bytes.push(byte);
            continue;
        }
        if !bytes.is_empty() {
            out.push_str(&String::from_utf8_lossy(&bytes));
            bytes.clear();
        }
        match token {
            "<s>" | "</s>" | "<unk>" => {}
            special if special.starts_with("<<ST_") => {}
            other => out.push_str(&other.replace(SPACE_MARK, " ")),
        }
    }
    if !bytes.is_empty() {
        out.push_str(&String::from_utf8_lossy(&bytes));
    }
    out
}

/// Downmix interleaved multi-channel samples to their per-frame average.
fn to_mono(samples: &[f32], channels: u16) -> Vec<f32> {
    let channels = usize::from(channels).max(1);
    if channels == 1 || samples.len() < channels {
        return samples.to_vec();
    }
    samples
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

/// One KV-cache split at the encoder/decoder boundary. The encoder
/// (cross-attention) half is frozen at the prefill values: the merged
/// decoder's cache branch emits constant-empty `present.*.encoder.*`.
struct KvCache {
    /// Cross-attention KV from the prefill, in slot order.
    encoder: Vec<KvSlot>,
    /// Rolling self-attention KV from the latest present.
    decoder: Vec<KvSlot>,
}

impl KvCache {
    /// Empty cache for the prefill pass.
    fn empty() -> Result<Self, VoiceError> {
        let all = empty_kv_cache()?;
        let (encoder, decoder) = KvCache::split(all);
        Ok(Self { encoder, decoder })
    }

    /// Partition a cache by slot kind, preserving order.
    fn split(all: Vec<KvSlot>) -> (Vec<KvSlot>, Vec<KvSlot>) {
        let mut encoder = Vec::new();
        let mut decoder = Vec::new();
        for (name, tensor) in all {
            if name.contains(".encoder.") {
                encoder.push((name, tensor));
            } else {
                decoder.push((name, tensor));
            }
        }
        (encoder, decoder)
    }

    /// Interleave both halves for a run; names are explicit per slot, so
    /// slide order does not matter to ORT.
    fn all(&self) -> impl Iterator<Item = (&String, &Tensor<f32>)> {
        self.encoder
            .iter()
            .chain(self.decoder.iter())
            .map(|(name, tensor)| (name, tensor))
    }

    /// Absorb the present outputs of one cache-branch step: the decoder
    /// (self-attention) slots always roll forward; encoder slots are adopted
    /// only when non-empty (this export's cache branch returns
    /// constant-empty cross-attention KV, so the prefill's stay in place).
    fn roll(&mut self, present: Vec<KvSlot>) -> Result<(), VoiceError> {
        let (present_encoder, present_decoder) = KvCache::split(present);
        for (slot, (_, tensor)) in self.encoder.iter_mut().zip(present_encoder) {
            let (shape, data) = tensor
                .try_extract_tensor::<f32>()
                .map_err(|e| VoiceError::Backend(format!("reading present encoder KV: {e}")))?;
            if !data.is_empty() {
                *slot = (
                    slot.0.clone(),
                    Tensor::from_array((shape.to_vec(), data.to_vec()))
                        .map_err(|e| VoiceError::Backend(format!("present encoder tensor: {e}")))?,
                );
            }
        }
        self.decoder = present_decoder;
        Ok(())
    }
}

/// A loaded Moonshine Tiny engine.
pub struct MoonshineEngine {
    encoder: Mutex<Session>,
    decoder: Mutex<Session>,
    /// BPE id-to-token table for greedy decoding.
    tokenizer: HashMap<i64, String>,
    decoder_start_token_id: i64,
    eos_token_id: i64,
}

impl MoonshineEngine {
    /// Load the engine from a model directory with the manifest layout
    /// (`onnx/encoder_model.onnx`, `onnx/decoder_model_merged.onnx`,
    /// `tokenizer.json`, `config.json`).
    ///
    /// Fails before touching ONNX Runtime when files are missing so the
    /// error names the fix.
    pub fn load(model_dir: &Path) -> Result<Self, VoiceError> {
        let encoder_path = model_dir.join(ENCODER_FILE);
        let decoder_path = model_dir.join(DECODER_FILE);
        let tokenizer_path = model_dir.join(TOKENIZER_FILE);
        let config_path = model_dir.join(CONFIG_FILE);
        for path in [&encoder_path, &decoder_path, &tokenizer_path, &config_path] {
            if !path.is_file() {
                return Err(VoiceError::Unavailable(format!(
                    "Moonshine Tiny STT files not found: {} is missing; run \
                     `alp-cli model download moonshine-tiny` to install them",
                    path.display()
                )));
            }
        }

        let tokenizer = parse_tokenizer(&tokenizer_path)?;
        let (decoder_start_token_id, eos_token_id) = parse_special_ids(&config_path)?;

        let mut encoder_builder = Session::builder(runtime_environment()).map_err(|e| {
            VoiceError::Backend(format!("ONNX Runtime session builder failed: {e}"))
        })?;
        let encoder = encoder_builder
            .commit_from_file(&encoder_path)
            .map_err(|e| VoiceError::Backend(format!("loading {}: {e}", encoder_path.display())))?;
        let mut decoder_builder = Session::builder(runtime_environment()).map_err(|e| {
            VoiceError::Backend(format!("ONNX Runtime session builder failed: {e}"))
        })?;
        let decoder = decoder_builder
            .commit_from_file(&decoder_path)
            .map_err(|e| VoiceError::Backend(format!("loading {}: {e}", decoder_path.display())))?;

        Ok(Self {
            encoder: Mutex::new(encoder),
            decoder: Mutex::new(decoder),
            tokenizer,
            decoder_start_token_id,
            eos_token_id,
        })
    }

    /// Encode 16 kHz mono samples into `[1, S, 288]` hidden states.
    fn encode(&self, samples: &[f32]) -> Result<(Vec<i64>, Vec<f32>), VoiceError> {
        let input = Tensor::from_array((vec![1, samples.len() as i64], samples.to_vec()))
            .map_err(|e| VoiceError::Backend(format!("audio tensor: {e}")))?;
        let mut session = self
            .encoder
            .lock()
            .map_err(|_| VoiceError::Backend("encoder session lock poisoned".into()))?;
        let outputs = session
            .run(vec![("input_values", SessionInputValue::from(input))])
            .map_err(|e| VoiceError::Backend(format!("encoder inference failed: {e}")))?;
        let (shape, hidden) = outputs["last_hidden_state"]
            .try_extract_tensor::<f32>()
            .map_err(|e| VoiceError::Backend(format!("reading encoder output: {e}")))?;
        Ok((shape.to_vec(), hidden.to_vec()))
    }

    /// One decoder pass: single token in, last-step logits + present out.
    fn decode_step(
        &self,
        token: i64,
        hidden: &[f32],
        hidden_shape: &[i64],
        use_cache_branch: bool,
        cache: &KvCache,
    ) -> Result<StepOutput, VoiceError> {
        let mut inputs: Vec<(&str, SessionInputValue)> = Vec::new();
        let ids = Tensor::from_array((vec![1, 1], vec![token]))
            .map_err(|e| VoiceError::Backend(format!("token tensor: {e}")))?;
        inputs.push(("input_ids", SessionInputValue::from(ids)));
        let states = Tensor::from_array((hidden_shape.to_vec(), hidden.to_vec()))
            .map_err(|e| VoiceError::Backend(format!("hidden tensor: {e}")))?;
        inputs.push((
            "encoder_hidden_states",
            SessionInputValue::from(states.view()),
        ));
        let branch = Tensor::from_array((vec![1], vec![use_cache_branch]))
            .map_err(|e| VoiceError::Backend(format!("branch tensor: {e}")))?;
        inputs.push(("use_cache_branch", SessionInputValue::from(branch.view())));
        for (name, tensor) in cache.all() {
            inputs.push((name.as_str(), SessionInputValue::from(tensor.view())));
        }

        let mut session = self
            .decoder
            .lock()
            .map_err(|_| VoiceError::Backend("decoder session lock poisoned".into()))?;
        let outputs = session
            .run(inputs)
            .map_err(|e| VoiceError::Backend(format!("decoder inference failed: {e}")))?;
        let (logits_shape, logits) = outputs["logits"]
            .try_extract_tensor::<f32>()
            .map_err(|e| VoiceError::Backend(format!("reading logits: {e}")))?;
        let logits_shape = logits_shape.to_vec();
        let logits = logits.to_vec();
        let present = kv_names()
            .into_iter()
            .map(|past_name| {
                let present_name = past_name.replace("past_key_values", "present");
                let value = &outputs[present_name.as_str()];
                let (shape, data) = value
                    .try_extract_tensor::<f32>()
                    .map_err(|e| VoiceError::Backend(format!("reading {present_name}: {e}")))?;
                let tensor = Tensor::from_array((shape.to_vec(), data.to_vec())).map_err(|e| {
                    VoiceError::Backend(format!("present tensor {present_name}: {e}"))
                })?;
                Ok((past_name, tensor))
            })
            .collect::<Result<Vec<_>, VoiceError>>()?;
        Ok((logits_shape, logits, present))
    }

    /// Greedy decode an encoded utterance into token ids.
    fn greedy_decode(&self, hidden: &[f32], hidden_shape: &[i64]) -> Result<String, VoiceError> {
        let mut cache = KvCache::empty()?;

        // Prefill: decoder start token, cache branch off, empty pasts.
        let (logits_shape, logits, present) = self.decode_step(
            self.decoder_start_token_id,
            hidden,
            hidden_shape,
            false,
            &cache,
        )?;
        let vocab = *logits_shape.last().unwrap_or(&0) as usize;
        let seq_len = *logits_shape.get(1).unwrap_or(&0) as usize;
        if vocab == 0 || seq_len == 0 {
            return Err(VoiceError::Backend("decoder produced empty logits".into()));
        }
        let first = argmax(&logits[(seq_len - 1) * vocab..seq_len * vocab]);
        let mut generated = vec![first];
        // The prefill produced valid cross-attention KV; freeze them and roll
        // only the self-attention half from here on.
        cache.roll(present)?;

        let mut next = first;
        for _ in 0..MAX_DECODE_STEPS {
            if next == self.eos_token_id {
                break;
            }
            let (logits_shape, logits, present) =
                self.decode_step(next, hidden, hidden_shape, true, &cache)?;
            let vocab = *logits_shape.last().unwrap_or(&0) as usize;
            let seq_len = *logits_shape.get(1).unwrap_or(&0) as usize;
            if vocab == 0 || seq_len == 0 {
                return Err(VoiceError::Backend("decoder produced empty logits".into()));
            }
            next = argmax(&logits[(seq_len - 1) * vocab..seq_len * vocab]);
            generated.push(next);
            cache.roll(present)?;
        }

        Ok(decode_ids(&self.tokenizer, &generated).trim().to_owned())
    }
}

impl SttBackend for MoonshineEngine {
    fn transcribe(&self, pcm: &PcmBuffer) -> Result<Vec<TranscriptSegment>, VoiceError> {
        if pcm.frames() == 0 {
            return Ok(Vec::new());
        }
        if u64::from(pcm.sample_rate()) * MAX_INPUT_SECS < pcm.frames() {
            return Err(VoiceError::Backend(format!(
                "capture is longer than {MAX_INPUT_SECS} s; trim it before transcribing"
            )));
        }

        let speech = resample(pcm, SAMPLE_RATE)
            .map_err(|e| VoiceError::Backend(format!("resampling to 16 kHz: {e}")))?;
        let mono = to_mono(speech.samples(), speech.channels());
        if mono.iter().all(|s| s.abs() < f32::EPSILON) {
            return Ok(Vec::new());
        }

        let (hidden_shape, hidden) = self.encode(&mono)?;
        let text = self.greedy_decode(&hidden, &hidden_shape)?;
        if text.is_empty() {
            return Ok(Vec::new());
        }
        Ok(vec![TranscriptSegment {
            start_frame: 0,
            end_frame: pcm.frames(),
            text,
        }])
    }

    fn streaming(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alp_voice::TtsBackend;
    use std::path::PathBuf;

    fn fixture_tokenizer(path: &Path) {
        std::fs::write(
            path,
            r#"{"model":{"type":"BPE","vocab":{"<unk>":0,"<s>":1,"</s>":2,
                "<0x41>":3,"<0x42>":4,"▁The":450,"▁fox":1187,"▁":29871}}}"#,
        )
        .expect("write tokenizer fixture");
    }

    fn fixture_config(path: &Path) {
        std::fs::write(
            path,
            r#"{"bos_token_id":1,"decoder_start_token_id":1,"eos_token_id":2,
                "hidden_size":288,"vocab_size":32768}"#,
        )
        .expect("write config fixture");
    }

    #[test]
    fn parse_tokenizer_reverses_the_bpe_vocab() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(TOKENIZER_FILE);
        fixture_tokenizer(&path);

        let table = parse_tokenizer(&path).expect("parse");
        assert_eq!(table.len(), 8);
        assert_eq!(table[&450].as_str(), "\u{2581}The");
        assert_eq!(table[&1187].as_str(), "\u{2581}fox");
    }

    #[test]
    fn parse_tokenizer_rejects_an_empty_or_bad_vocabulary() {
        let dir = tempfile::tempdir().expect("tempdir");
        let bad = dir.path().join("bad.json");
        std::fs::write(&bad, r#"{"model":{"vocab":{}}}"#).expect("write bad");
        assert!(parse_tokenizer(&bad).is_err());

        let wrong = dir.path().join("wrong.json");
        std::fs::write(&wrong, r#"{"something":{}}"#).expect("write wrong");
        assert!(parse_tokenizer(&wrong).is_err());
    }

    #[test]
    fn parse_special_ids_reads_decoder_start_and_eos() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(CONFIG_FILE);
        fixture_config(&path);
        let (start, eos) = parse_special_ids(&path).expect("parse");
        assert_eq!((start, eos), (1, 2));

        let missing = dir.path().join("missing.json");
        std::fs::write(&missing, r#"{"hidden_size":288}"#).expect("write partial");
        assert!(parse_special_ids(&missing).is_err());
    }

    #[test]
    fn decode_ids_maps_spaces_bytes_and_skips_specials() {
        let table: HashMap<i64, String> = [
            (0, "<unk>".into()),
            (1, "<s>".into()),
            (2, "</s>".into()),
            (3, "<0x41>".into()), // 'A'
            (4, "<0x42>".into()), // 'B'
            (450, "\u{2581}The".into()),
            (29871, "\u{2581}".into()),
        ]
        .into_iter()
        .collect();

        let text = decode_ids(&table, &[1, 450, 29871, 1187, 3, 4, 2]);
        // specials dropped, space marks resolved, byte pair fused to "AB".
        // The fox id is absent from this tiny fixture and decodes to "".
        // The leading space is faithful BPE output ("`<0x2581>`The" starts
        // with the space mark); `greedy_decode` trims the final text.
        assert_eq!(text, " The AB");
    }

    #[test]
    fn to_mono_averages_interleaved_frames() {
        let stereo = vec![0.0f32, 1.0, 0.5, 0.25];
        assert_eq!(to_mono(&stereo, 2), vec![0.5, 0.375]);
        let mono = vec![0.7f32, 0.1];
        assert_eq!(to_mono(&mono, 1), vec![0.7, 0.1]);
        // A buffer shorter than one frame passes through untouched.
        assert_eq!(to_mono(&mono, 4), vec![0.7, 0.1]);
    }

    #[test]
    fn kv_slot_names_cover_every_layer_part_in_order() {
        let names = kv_names();
        assert_eq!(names.len(), 24);
        assert_eq!(names[0], "past_key_values.0.decoder.key");
        assert_eq!(names[3], "past_key_values.0.encoder.value");
        assert_eq!(names[23], "past_key_values.5.encoder.value");
        let empty = empty_kv_cache().expect("empty cache");
        for (name, tensor) in &empty {
            assert!(name.starts_with("past_key_values."));
            let (shape, data) = tensor.try_extract_tensor::<f32>().expect("read");
            assert_eq!(shape.len(), 4);
            assert_eq!(data.len(), 0, "{name} must have an empty time axis");
        }
    }

    #[test]
    fn load_fails_honestly_when_files_are_absent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let error = MoonshineEngine::load(&dir.path().join("moonshine-tiny"))
            .err()
            .expect("load must fail");
        match error {
            VoiceError::Unavailable(message) => {
                assert!(message.contains("moonshine-tiny"), "{message}");
                assert!(message.contains("alp-cli model download"), "{message}");
            }
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    /// End-to-end smoke against the real downloaded model: kitten speaks a
    /// known phrase (self-contained ground truth), we resample to 16 kHz,
    /// and Moonshine must read it back. Skipped when either model is absent
    /// (standard XDG location or `ALP_MOONSHINE_DIR` / `ALP_KITTEN_DIR`).
    #[test]
    fn real_model_transcribes_speech_when_installed() {
        let alp_dir = || alp_voice::ModelManagerConfig::default().storage_dir.clone();
        let dir = std::env::var("ALP_MOONSHINE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| alp_dir().join("moonshine-tiny"));
        if !dir.join(ENCODER_FILE).is_file() || !dir.join(DECODER_FILE).is_file() {
            eprintln!("skip: moonshine-tiny not installed at {}", dir.display());
            return;
        }
        let kitten_dir = std::env::var("ALP_KITTEN_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| alp_dir().join("kitten-nano"));
        if !kitten_dir.join("onnx/model_quantized.onnx").is_file() {
            eprintln!("skip: kitten-nano (probe audio) not installed");
            return;
        }

        let engine = MoonshineEngine::load(&dir).expect("load moonshine");
        let spoken = crate::kitten::KittenEngine::load(&kitten_dir)
            .expect("load kitten")
            .synthesize(&alp_voice::TtsRequest {
                text: "The quick brown fox jumps over the lazy dog.".into(),
                voice_id: "expr-voice-2-f".into(),
                speed: 1.0,
            })
            .expect("kitten synthesize");
        let speech = resample(&spoken, SAMPLE_RATE).expect("resample");
        let mono = PcmBuffer::new(SAMPLE_RATE, 1, speech.samples().to_vec()).expect("mono pcm");

        let started = std::time::Instant::now();
        let segments = engine.transcribe(&mono).expect("transcribe");
        let elapsed = started.elapsed();
        eprintln!(
            "moonshine smoke: {} segment(s) in {elapsed:?}: {:?}",
            segments.len(),
            segments.first().map(|s| s.text.as_str())
        );

        assert_eq!(segments.len(), 1, "no-timestamp engines emit one segment");
        let segment = &segments[0];
        assert_eq!(segment.start_frame, 0);
        assert_eq!(segment.end_frame, mono.frames());
        let normalized = segment.text.to_lowercase();
        assert!(
            normalized.contains("quick") && normalized.contains("fox"),
            "expected the phrase content, got {:?}",
            segment.text
        );
        assert!(
            !normalized.contains("placeholder"),
            "must be real speech text, got {:?}",
            segment.text
        );
    }

    /// Silence must produce no bogus segments.
    #[test]
    fn silence_yields_no_segments_when_installed() {
        let dir = std::env::var("ALP_MOONSHINE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                alp_voice::ModelManagerConfig::default()
                    .storage_dir
                    .join("moonshine-tiny")
            });
        if !dir.join(ENCODER_FILE).is_file() {
            eprintln!("skip: moonshine-tiny not installed");
            return;
        }
        let engine = MoonshineEngine::load(&dir).expect("load");
        let silence = PcmBuffer::new(SAMPLE_RATE, 1, vec![0.0; SAMPLE_RATE as usize]).expect("pcm");
        let segments = engine.transcribe(&silence).expect("transcribe silence");
        assert!(
            segments.is_empty(),
            "silence must not transcribe: {segments:?}"
        );
    }

    /// An absurdly long capture is refused with guidance, not decoded.
    #[test]
    fn overlong_buffers_are_refused_when_installed() {
        let dir = std::env::var("ALP_MOONSHINE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                alp_voice::ModelManagerConfig::default()
                    .storage_dir
                    .join("moonshine-tiny")
            });
        if !dir.join(ENCODER_FILE).is_file() {
            eprintln!("skip: moonshine-tiny not installed");
            return;
        }
        let engine = MoonshineEngine::load(&dir).expect("load");

        // 301 s of silence at 8 kHz: 2,408,000 frames clears the
        // 300 s / 2,400,000-frame ceiling without materializing 16 kHz
        // audio (2.4M f32, about 10 MB).
        let pcm = PcmBuffer::new(8_000, 1, vec![0.0; 8_000 * 301]).expect("pcm");
        assert_eq!(pcm.frames(), 8_000 * 301);

        let error = engine.transcribe(&pcm).expect_err("must refuse");
        let message = error.to_string();
        assert!(message.contains("trim"), "{message}");
    }
}
