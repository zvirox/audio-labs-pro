# ADR 0023: Shared StyleTTS2 Engine Layer and the Kitten Nano Wiring

- **Status:** Accepted
- **Date:** 2026-10-29
- **Deciders:** Audio Labs Pro Engineering
- **Consulted:** `onnx-community/kitten-tts-nano-0.1-ONNX` export, `KittenML/kitten-tts-nano-0.1` model card, `onnx-community/Kokoro-82M-v1.0-ONNX` tokenizer formats, local verified downloads

## Context and Problem Statement

Phase 6 shipped exactly one neural engine: `KokoroEngine` (ADR 0008/0010), loaded and activated by the `alp-infer` daemon's `LoadModel`. The catalogue's second TTS model, `kitten-nano` (Apache-2.0, ~24 MB quantized), is downloaded and hash-verified but the daemon declines its load with "no engine is wired for this model yet", while the UI (ADR 0022 follow-on) offers a *Load engine* button for it. Both models share the `style_text_to_speech_2` architecture family (phoneme ids + 256-lane style vector + speed into a waveform), so hosting two engines must not mean duplicating the ONNX boilerplate.

## Decision Drivers

1. **Honest surfaces:** once the UI promises *Load engine*, the daemon must either load the model or refuse with the exact reason, never a generic failure.
2. **License purity:** kitten-nano is Apache-2.0; phonemization must stay on the MIT `piper-plus-g2p` path (no espeak-ng), per ADR 0010.
3. **Evidence over guessing:** sample rate, tensor signature, padding id, and voice-vector layout come from the upstream card and the locally verified files, not hearsay.
4. **No duplicate ONNX plumbing:** two engines in one daemon require genuinely shared code, not copy-paste drift.
5. **Synchronous, single-threaded daemon loop:** the engine pair must fit the existing `TtsEngine` enum and its sequential accept loop.

## Considered Options

- **Option 1: Shared `style_tts2` helper layer + thin per-model engines (Chosen)**
  - Extract what is genuinely common (vocabulary parse, IPA-to-ids with `$`=0 padding, style-row selection, session bootstrap + tensor-signature validation, chunked inference) into `style_tts2.rs`; `kokoro.rs` and `kitten.rs` keep only model facts.
  - Evidence: kitten's `tokenizer.json` is an HF `tokenizers` file whose `model.vocab` is the same single-character IPA map the Kokoro parser already reads (`$`=0, 175 tokens); voices are one 256-f32 row per voice (`voices/*.bin`, 1,024 bytes each); the upstream card writes audio at 24 kHz.
  - `TtsEngine::Neural` widens from `Box<KokoroEngine>` to `Box<dyn TtsBackend>` so the daemon's load path is a table of engines, not an if-chain growth.
- **Option 2: Duplicate `kitten.rs` wholesale from `kokoro.rs`**
  - Fastest, but forks the parser, padding, and style logic; the next TTS model would be a third copy.
- **Option 3: Generic "transformers.js TTS" runner driven by `config.json` metadata**
  - Rejected for now: the export's `config.json` carries almost no executable metadata (no sample rate, no tensor names), so the abstraction would be driven by hand-written tables anyway with extra indirection.

## Decision

1. Add `crates/alp-infer/src/style_tts2.rs` holding the common engine mechanics; `KokoroEngine` and the new `KittenEngine` both delegate to it.
2. Add `crates/alp-infer/src/kitten.rs`: `MODEL_ID = "kitten-nano"`, `SAMPLE_RATE = 24_000`, weights resolved in the order `onnx/model_quantized.onnx`, `onnx/model.onnx`, `model_quantized.onnx`, `model.onnx`; phonemization via `EnglishPhonemizer::new_bundled()` (ADR 0010); token ids 0-padded at both ends (`$` is id 0, same convention as Kokoro); voice style = the single 256-f32 row in `voices/<stem>.bin`; voices are the eight `expr-voice-{2..5}-{f,m}` vectors, with display names resolved canonically (letters/digits) so `Expr Voice 2 (F)` matches `expr-voice-2-f.bin`; default voice `expr-voice-2-f`.
3. Tensor signature is validated at load time: `input_ids` and `style` are required; `speed` is bound only when the export provides it. Unknown exports fail with the found signature named.
4. `TtsEngine::Neural` becomes `Box<dyn TtsBackend>`; `load_model_from` maps `kokoro-82m` and `kitten-nano` to their engines and declines anything else with its honest message. A failed load keeps the previous engine (existing contract, test-pinned).
5. Voice fallback follows the Kokoro pattern: a request naming an uninstalled voice renders with the engine's default voice.

## Consequences

- **Positive:** installing kitten-nano is now sufficient for the UI's *Load engine* to succeed; both engines share one tested inference path; a third StyleTTS2-family model is a table entry.
- **Positive:** quantized (`model_quantized.onnx`) weights keep the daemon's load under a second on CPU.
- **Neutral:** KittenTTS nano v0.1 is a developer-preview model; quality questions are upstream's, and the honest-refusal path remains for load errors.
- **Negative:** the `expr-voice-*` naming carries no gender metadata in the file names beyond the `-f`/`-m` suffix; the resolver's canonical pass must not pretend to know more than the catalogue does (display names come from the manifest, not guesses).
- **Out of scope, unchanged:** `moonshine-tiny` is `ModelKind::Stt`; wiring it belongs to the Phase 7 dictation slice. The D3 heavy tier (Qwen3-TTS et al.) still needs its own downloads and decision.

## Verification

- Fixture tests cover vocabulary parsing on both tokenizer layouts, 0-padding, single-row style selection, canonical voice resolution, and honest load failures.
- An installed-model smoke test (skipped when absent, like the Kokoro one) asserts non-silent 24 kHz mono speech through the real quantized graph.
- Worker tests pin the load transitions: kitten loads when its files exist, declines without them, and never disturbs the active engine on failure.
