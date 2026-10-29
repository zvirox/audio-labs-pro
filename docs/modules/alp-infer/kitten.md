# `alp-infer` / `kitten`

## Purpose

The neural TTS engine for Kitten Nano v0.1 (Apache-2.0, `onnx-community/
kitten-tts-nano-0.1-ONNX`, quantized graph ~24 MB), executed through the
shared `style_text_to_speech_2` layer (ADR 0023). Wired so the UI's
*Load engine* affordance on an installed kitten-nano genuinely activates the
model in the `alp-infer` daemon.

## Public API

- `KittenEngine::load(model_dir)`: validates the directory layout
  (`onnx/model_quantized.onnx` with `model.onnx` fallbacks, `tokenizer.json`,
  `voices/*.bin`) before touching ONNX Runtime.
- `TtsBackend` impl (`id = "kitten-nano"`): `voices` lists the eight
  `expr-voice-{2..5}-{f|m}` style bins with manifest display names;
  `synthesize` delegates to the shared chunked flow (ADR 0024).
- `SAMPLE_RATE` (24 kHz mono, per the KittenML model card), `MODEL_ID`.

## Data flow

1. `synthesize` delegates to `style_tts2::synthesize_text`: normalize, split
   long text into sentence chunks (ADR 0024), phonemize through
   `piper-plus-g2p` bundled CMUdict (ADR 0010), map through `tokenizer.json`'s
   `model.vocab` (`$` = 0 padding; unknown characters logged).
2. Style vectors are single 256-f32 rows; the canonical voice pass resolves
   manifest display names ("Expr Voice 4 (M)") to `expr-voice-4-m.bin`;
   unknown voices fall back to `expr-voice-2-f`.
3. One ONNX run per chunk (`input_ids` + `style` always; `speed` bound only
   when the graph provides it), chunks concatenated with 0.15 s silences into
   a 24 kHz mono `PcmBuffer`.

## Allowed dependencies

- `alp-media` (`PcmBuffer`), `alp-voice` (`TtsBackend`, `VoiceError`),
  `piper-plus-g2p` (MIT), sibling module `style_tts2`; no new external
  dependencies beyond what the daemon already carries.

## Threading / real-time notes

Same contract as `kokoro.md`: worker-process only, never audio callbacks;
sessions live behind the shared `NeuralSession` mutex on the daemon's
sequential accept loop.

## Error handling

Missing weights/vocabulary/voices produce `VoiceError::Unavailable` messages
naming the file and `alp-cli model download kitten-nano`; an unreadable
tokenizer or invalid graph is a `VoiceError::Backend` load failure, and the
daemon keeps its previous engine (test-pinned). Unknown voices fall back to
the default voice.

## Testing

Fixture tests pin voice resolution (manifest display names, bare stems,
unknown misses) and honest load failures. The gated smoke test
`real_model_synthesizes_speech_when_installed` (XDG dir or `ALP_KITTEN_DIR`)
asserts non-silent 24 kHz mono speech; `long_text_is_reconstructed_from_chunks_when_installed`
asserts the shared chunked flow renders the full paragraph through the real
model.

## Known limits

- KittenTTS nano v0.1 is an upstream developer-preview model; voice quality
  is upstream's, not tunable here.
- The 8 style bins are the shy/normal/expressive expr-voice presets; no free
  voice design exists for this engine.
- English only, `piper-plus-g2p` English phoneme set mapped onto kitten's
  IPA vocabulary; characters outside it are skipped (logged), never fatal.
