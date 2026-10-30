# ADR 0025: Moonshine Tiny STT Engine and the Two-Tier Daemon Contract

- **Status:** Accepted
- **Date:** 2026-10-29
- **Deciders:** Audio Labs Pro Engineering
- **Consulted:** `onnx-community/moonshine-tiny-ONNX` export (MIT), local verified download, `huggingface/transformers.js` generation loop (upstream reference runner), ADR 0023 (shared engine layer)

## Context and Problem Statement

Phase 6 closed with exactly one kind of neural engine in the daemon: synthesis. The STT tier is still the built-in energy/formant segmenter whose output is honest placeholder words, and the UI says so. The catalogue already contains `moonshine-tiny` (MIT, ~107 MB), downloadable through the verified pipeline, and Phase 7's flagged gap from ADR 0023 is wiring it as the first neural transcription engine without disturbing the synthesis tier that ADR 0023/0024 established.

## Decision Drivers

1. **License purity:** moonshine-tiny-ONNX is MIT (same terms as the rest of the stack); no new dependencies, still `ort` 2.0.0-rc.15 CPU-only per ADR 0008/0010.
2. **Single ONNX Runtime environment:** rc.15 permits exactly one live `Environment` per process; a second init aborts the process (probe-verified). The STT engine must share the TTS engines' global.
3. **Independent tiers:** loading a transcription model must never unload a loaded synthesis model, and the user must see that state honestly (`Status` carries both).
4. **Honest surfaces:** the studio transcription toast must name the engine that ran and must keep the placeholder-words admission whenever the energy/formant segmenter - daemon-side or in-process - did the transcribing.
5. **Evidence over guessing:** the merged-decoder calling convention is pinned empirically against the installed export, not inferred.

## Considered Options

- **Option 1: Dedicated `moonshine.rs` engine behind the existing `SttBackend` trait, second daemon slot (Chosen)**
  - `MoonshineEngine` implements `SttBackend::transcribe` directly (16 kHz mono in, one whole-buffer segment out, no timestamps); the daemon gains an `SttEngine` enum mirroring `TtsEngine`.
  - Evidence: the export is an encoder/decoder pair of ONNX graphs with a Llama-style BPE (`tokenizer.json`, specials `<s>`=1/`</s>`=2, byte-fallback `<0xNN>`, `▁`=space, `<<ST_n>>` tail) and a merged decoder whose real-branch cache semantics were probed end to end (kitten speaks a known phrase; Moonshine reads it back).
- **Option 2: Reuse `SttEngine = BuiltinSttBackend` and swap the VAD config for neural output**
  - Rejected: a neural transcriber is not a VAD tuner; the trait already abstracts the boundary cleanly.
- **Option 3: Generic "audio-encoder/seq2seq" runner family like `style_tts2`**
  - Rejected for now: Moonshine is the first STT engine, so there is nothing to share yet; a second STT model would justify extraction (the ADR 0023 pattern applied when the second TTS engine arrived).

## Decision

1. **Engine (`crates/alp-infer/src/moonshine.rs`):** `MoonshineEngine::load(model_dir)` requires `onnx/encoder_model.onnx`, `onnx/decoder_model_merged.onnx`, `tokenizer.json`, `config.json`, and fails *before* touching ONNX Runtime when files are missing ("run `alp-cli model download moonshine-tiny`"). Guards: input longer than 300 s is refused with trim guidance; all-silent input returns no segments; `MAX_DECODE_STEPS` = 448 caps greedy decoding.
2. **Decode convention (empirically pinned, test-pinned):**
   - Encoding: `input_values` f32 `[1, N]` (16 kHz mono, mean-mixed channels via `alp_media::resample` + mixdown) -> `last_hidden_state` `[1, S, 288]`.
   - Prefill: decoder run with `input_ids=[[decoder_start_token_id(=1)]]`, 24 empty KV pasts `[1, 8, 0, 36]`, `use_cache_branch=[false]`; logits `[1, 1, 32768]`, argmax = first token.
   - Steps: one token per run with `use_cache_branch=[true]` and the previous `present.*` as pasts; stop at `eos_token_id(=2)`.
   - **Frozen cross-attention KV:** the merged decoder's cache branch emits `present.*.encoder.*` as constant-empty tensors (shape `[0, 8, 1, 36]`, a literal Constant node in the export). Feeding them back fails at `/model/decoder/layers.0/encoder_attn/MatMul` ("right operand cannot broadcast on dim 0"). Therefore `KvCache` splits encoder (cross-attn) from decoder (self-attn) slots: the encoder half is taken from the prefill and never replaced; only the 12 decoder slots roll forward. `KvCache::roll` adopts non-empty encoder presents if a future export fixes this.
3. **Global ORT environment correction:** `style_tts2::runtime_environment()` becomes `pub(crate)` and is the *only* initializer; `moonshine.rs` and the test scaffolding reuse it. The probe run that crashed with two environments is the pinned evidence (rc.15, one live `Environment` per process).
4. **Two-tier daemon contract:**
   - `SttEngine { Builtin(BuiltinSttBackend), Neural(Box<dyn SttBackend>) }` mirrors `TtsEngine`; `Worker` carries `stt` + `active_stt: Option<String>` beside `engine` + `active_model`.
   - `load_model_from("moonshine-tiny", dir)` loads into the STT slot only. `builtin-acoustic` resets synthesis only - the built-in STT is shadowed, never unloaded.
   - `InferResponse::Status` gains `active_stt_model: Option<String>`; the new `InferStatus { active_model, ready, active_stt_model }` struct is what `client.status()` / `client.load_model()` return. `ready` still refers to the last `LoadModel` attempt; slot ids only ever change on success.
5. **Worker → UI contract:** `EngineState` gains `active_stt_model`; the Neural Tier card lists `ModelKind::Stt` rows and judges each row's *active engine* chip by its own slot (a declined load of one model cannot un-chip an engine that still serves). `transcribe_capture` goes daemon-first: daemon STT when alive (toast names the engine via the same daemon's status), in-process built-in otherwise - the placeholder-words admission stays wherever the acoustic segmenter ran.
6. **Transcript convention:** one whole-buffer segment (`start_frame: 0, end_frame: pcm.frames()`), no timestamps, `streaming() == false` - matching the CLI's existing expectations.

## Consequences

**Positive**

- The daemon now serves both directions: neural synthesis and neural transcription coexist, each independently loadable, with built-in fallbacks on both tiers.
- The UI's honest-state pattern extends to transcription: toasts and chips name what actually ran.
- upstream-independent: no new crates; the runtime stays rc.15 CPU; the export's quirk is handled in our code, documented, and test-pinned against the real weights.

**Negative / follow-ups**

- The `use_cache_branch` frozen-encoder-KV convention is an export artifact; a future re-export may change it (the adoption code handles the non-empty case already; tests will catch the drift).
- Transcription is non-streaming and capped at 300 s per call; long captures need chunked capture-side handling (candidate for a later phase).
- `Status.ready` remains a single "last load attempt" flag across two slots; per-slot readiness would need another protocol extension.

## References

- `crates/alp-infer/src/moonshine.rs` - engine, decode convention, gated real-model smoke test
- `crates/alp-infer/src/main.rs` - `SttEngine`, two-tier `Worker`, gated two-slot test
- `crates/alp-voice/src/protocol.rs` - `InferStatus`, extended `Status`
- `crates/alp-ui/src/app/model_actions.rs`, `voice_views.rs`, `studio.rs` - UI contract
