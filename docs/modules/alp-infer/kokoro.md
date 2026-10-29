# `alp-infer` / `kokoro`

## Purpose

The neural TTS engine for Kokoro-82M (v1.0 fp32 ONNX export), executed through
ONNX Runtime (`ort`, CPU execution provider per ADR 0008) with a pure-Rust,
espeak-free text front-end via `piper-plus-g2p` and its bundled CMUdict
(ADR 0010). All `style_text_to_speech_2` mechanics are shared with the Kitten
engine through `style_tts2.rs` (ADR 0023); this module keeps only Kokoro
model facts.

## Public API

- `KokoroEngine::load(model_dir)`: validates the model directory layout
  (`onnx/model.onnx`, `tokenizer.json`, `voices/*.bin`) before touching ONNX
  Runtime, then builds the session and input-name check.
- `TtsBackend` impl (`id = "kokoro-82m"`): `voices` lists installed style
  bins with friendly names; `synthesize` normalizes text, phonemizes to IPA,
  maps tokens, selects the style row by token count, and runs inference.
- `SAMPLE_RATE` (24 kHz mono), `MODEL_ID`.

## Data flow

1. `synthesize` delegates to `style_tts2::synthesize_text`: the request is
   normalized, long text is split into sentence chunks (ADR 0024), each chunk
   is phonemized (IPA) and mapped through the character vocabulary parsed
   from `tokenizer.json` (0-padded both ends; unknown characters are skipped
   and logged).
2. The style vector is read from `voices/<voice>.bin`, row
   `min(chunk_token_count, rows - 1)` (256 f32 lanes per row, chosen per
   chunk so prosody matches the chunk), falling back to the default voice
   when the request names an unknown one.
3. Each chunk runs one ONNX inference with inputs `input_ids` (`[1, N] i64`),
   `style` (`[1, 256] f32`), `speed` (`[1] f32`, clamped 0.5-2.0); the chunk
   waveforms are concatenated with 0.15 s silences into a 24 kHz mono
   `PcmBuffer`.

## Allowed dependencies

- `alp-media` (`PcmBuffer`), `alp-voice` (`TtsBackend`, `VoiceError`, `TextNormalizer`)
- `ort` (pinned `2.0.0-rc.15`), `piper-plus-g2p` (MIT; `bundled-dicts` for CMUdict)
- `serde_json` (tokenizer parsing), standard library

## Threading / real-time notes

Runs only inside the `alp-infer` worker, never in an audio callback. The ONNX
session requires `&mut` for `run`, so it sits behind a `Mutex<Session>`; the
daemon serves connections sequentially, so the lock is uncontended in
practice, and the session stays idle between requests.

## Error handling

Missing model files produce `VoiceError::Unavailable` messages naming the
expected file and how to install it; G2P, tokenizer and inference failures
surface as `VoiceError::Backend`. Unknown voices fall back to the default
style, and truly empty phonemization is an error, not silence.

## Testing

Unit tests run without ONNX Runtime (vocabulary parsing, token padding,
voice-path resolution, style-row selection, voice listing). The gated smoke
test `real_model_synthesizes_speech_when_installed` runs real inference when
the model is installed under the XDG model directory or `ALP_KOKORO_DIR` and
asserts non-silent, speech-quality audio; it skips when the model is absent.

## Known limits

- Style-vector selection is a length-bucket heuristic, not Kokoro's
  token-level style pipeline.
- English only (`piper-plus-g2p` English feature); other languages need
  additional G2P front-ends and voice packs.
- The three curated voices are whatever style bins are installed; only the
  three manifest voices are hash-verified upstream.
- Cancellation between chunks of a long synthesis is not implemented; the
  daemon's per-request cancel has room for it later.
