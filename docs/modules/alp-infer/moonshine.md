# `alp-infer` / `moonshine`

## Purpose

Moonshine Tiny neural speech-to-text engine (ADR 0025): ONNX encoder/decoder
inference over 16 kHz mono audio with greedy BPE decoding, served through the
`SttBackend` trait. First engine of the daemon's transcription tier; the
synthesis tier (`kokoro.rs`, `kitten.rs` via `style_tts2.rs`) is untouched by
its loads and vice versa.

## Public API

- `MODEL_ID = "moonshine-tiny"`, `SAMPLE_RATE = 16_000`
- `MoonshineEngine::load(model_dir)` - requires the manifest layout
  (`onnx/encoder_model.onnx`, `onnx/decoder_model_merged.onnx`,
  `tokenizer.json`, `config.json`); fails before touching ONNX Runtime when
  files are missing, naming the `alp-cli model download moonshine-tiny` fix
- `SttBackend for MoonshineEngine`: `transcribe` resamples to 16 kHz, mixes
  channels to mono, encodes, greedy-decodes, and emits ONE whole-buffer
  `TranscriptSegment` in the caller's frame space (no timestamps);
  `streaming()` is `false`

## Data flow

1. `transcribe(pcm)` guards length (300 s ceiling, trim guidance) and
   silence (no segments), then resamples to 16 kHz (`alp_media::resample`)
   and mean-mixes channels to mono.
2. Encoder: `input_values` f32 `[1, N]` -> `last_hidden_state` `[1, S, 288]`.
3. Decoder prefill: `input_ids=[[1]]` (`decoder_start_token_id` from
   `config.json`), 24 empty KV pasts `[1, 8, 0, 36]`,
   `use_cache_branch=[false]`; argmax over the last logits row
   (`[1, 1, 32768]`) yields the first token.
4. Greedy steps: one token per run with `use_cache_branch=[true]`, feeding
   the previous `present.*` as pasts; stop at `eos_token_id(=2)`, capped at
   `MAX_DECODE_STEPS = 448`.
5. **Cross-attention KV freeze (ADR 0025):** the merged decoder's cache
   branch emits constant-empty `present.*.encoder.*` (a Constant node of
   shape `[0, 8, 1, 36]`); feeding them back fails cross-attention MatMul.
   `KvCache` therefore fixes the encoder half to the prefill's tensors and
   rolls only the 12 self-attn slots; non-empty encoder presents are adopted
   (future-export safety).
6. BPE decode: id-to-token table from `tokenizer.json`; specials skipped,
   `<0xNN>` byte-fallback runs fused as UTF-8, `\u{2581}` mapped to space;
   the text is trimmed and becomes one whole-buffer segment spanning
   `0..pcm.frames()`.

## Allowed dependencies

- `ort` 2.0.0-rc.15 CPU (shared process-wide environment via
  `style_tts2::runtime_environment`; rc.15 allows one live `Environment`)
- `alp_media` (`resample`, `PcmBuffer`), `alp_voice`
  (`SttBackend`, `TranscriptSegment`, `VoiceError`)
- `serde_json` for tokenizer/config reads

## Threading / real-time notes

- Engine methods run on the daemon's sequential accept loop; the ONNX
  sessions sit behind `Mutex` so `&self` suffices and a second connection
  cannot interleave runs. Poisoned locks surface as `VoiceError::Backend`.
- Never call from an audio callback: model inference is hundreds of
  milliseconds of CPU work.

## Error handling

- `VoiceError::Unavailable` when model files are missing (before ORT init),
  with the download command in the message.
- `VoiceError::Backend` for tensor construction/extraction, session lock,
  inference failures, malformed tokenizer/config, and the overlong-buffer
  refusal (message says "trim it before transcribing").

## Testing

- fixture unit tests: tokenizer/config parsing (empty and wrong layouts
  rejected), BPE decode edge cases (specials, byte runs, space marks), mono
  mixdown, KV slot inventory (24 slots, empty time axis)
- `load_fails_honestly_when_files_are_absent`: error names the fix
- gated real-model tests (skipped without the download):
  `real_model_transcribes_speech_when_installed` (kitten speaks the ground
  truth phrase, Moonshine reads it back),
  `silence_yields_no_segments_when_installed`,
  `overlong_buffers_are_refused_when_installed`
- worker-level two-slot test in `main.rs`: kitten (synthesis) and Moonshine
  (transcription) load independently and the `builtin-acoustic` reset touches
  synthesis only

## Known limits

- non-streaming: the whole buffer is encoded in one pass
- a 300 s ceiling per transcription call
- greedy decoding only (no beam search / temperature)
- the frozen-encoder-KV handling is pinned to this export family; a
  re-export that grows cross-attention KV through the cache branch is
  adopted by `KvCache::roll` but should be re-verified against the
  round-trip smoke test
