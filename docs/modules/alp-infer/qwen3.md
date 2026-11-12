# `alp-infer` / `qwen3`

## Purpose

The heavy neural voice cloning engine for Qwen3-TTS 0.6B (Apache-2.0,
Alibaba Cloud / Qwen), executed within the out-of-process `alp-infer` worker
daemon (ADR 0008, ADR 0018).

## Status

**The installation is real and validated.** The engine checks that the
Apache-2.0 GGUF weights are present, that they really are GGUF containers,
and that they carry the `qwen3-tts` architecture with the expected talker
depth.

**The inference runtime is not yet implemented.** Qwen3-TTS is a bespoke
three-part architecture — a Res2Net speaker encoder (`spk_enc`), a Qwen3
talker with multi-axis rotary embeddings, and a code predictor (`code_pred`,
5 blocks) — plus a separate 12 Hz speech-token decoder. Nothing in the tree
can execute it today, so this engine reports that honestly instead of
producing synthetic audio that could be mistaken for neural output.

Until the runtime lands, cloned voices are rendered by the acoustic tier
(`alp_voice::AcousticCloneBackend`), which measures the reference's
voiceprint and reproduces its timbre. The consent gate, profile storage,
`.alpvoice` bundles and daemon routing are identical for both tiers.

## Public API

- `Qwen3Engine::load(model_dir) -> Result<Self, VoiceError>`: validates the
  installation layout (`gguf/qwen-talker-0.6b-base-Q4_K_M.gguf`,
  `gguf/qwen-tokenizer-12hz-Q4_K_M.gguf`, `tokenizer_config.json`,
  `vocab.json`, `merges.txt`, `config.json`) and returns
  `VoiceError::Unavailable` with user-actionable download commands if
  required weights are absent.
  - The talker's GGUF header is parsed and must declare the architecture
    `qwen3-tts`; a mismatch is a `VoiceError::Backend`.
  - When the talker declares a block count, it must be 28 (the 0.6B
    talker's depth).
  - The decoder is a separate container and must also be readable and
    declare an architecture.
- `Qwen3Engine::model_dir()`, `talker_path()`, `decoder_path()`: the
  validated installation paths.
- `Qwen3Engine::extract_embedding(reference) -> Vec<f32>`: a 256-dimensional
  speaker descriptor derived from the reference's measured voiceprint.
- `TtsBackend` implementation (`id = "qwen3-tts"`): lists the supported voice
  descriptors; `synthesize` reports the unavailable runtime.
- `CloneBackend` implementation:
  - `create_profile(reference, transcript)`: generates a `VoiceProfile` of
    kind `Cloned` containing the speaker descriptor. The consent record is
    always `None` — the engine never fabricates one.
  - `synthesize_with(profile, request)`: reports the unavailable runtime.
- Constants: `MODEL_ID = "qwen3-tts"`, `SAMPLE_RATE = 24_000`.

## Data flow

1. **Load.** `Qwen3Engine::load` checks that every expected file exists.
   Missing files produce `VoiceError::Unavailable` naming the file and the
   `alp-cli model download qwen3-tts` command.
2. **Validate.** The talker's GGUF header is parsed by
   [`gguf`](./gguf.md). The architecture must be `qwen3-tts`, and the talker
   block count must be 28. The decoder header must also be readable.
3. **Dispatch.** In `alp-infer`, a successful load puts the engine in the
   daemon's *clone* slot, leaving the synthesis and transcription slots
   untouched. A failed load keeps the acoustic clone tier active.
4. **Render.** `synthesize` and `synthesize_with` return
   `VoiceError::Unavailable` naming the missing runtime. No audio is
   produced, so a caller can never mistake this engine's output for neural
   speech.

## Allowed dependencies

- `alp-media`: `PcmBuffer`
- `alp-voice`: `TtsBackend`, `CloneBackend`, `TtsRequest`, `VoiceProfile`,
  `VoiceError`, `VoiceInfo`, `VoiceParams`, `VoiceKind`
- `super::gguf`: the GGUF header reader
- Safe Rust only (`unsafe_code = "deny"`).

## Threading / real-time notes

Daemon-worker process only, never on the real-time audio callback. Instances
are loaded and dispatched on the sequential accept loop of `alp-infer`.

## Error handling

- Missing weight files, tokenizers or configuration produce
  `VoiceError::Unavailable` detailing the missing file and instructing the
  user to run `alp-cli model download qwen3-tts`.
- A wrong architecture or talker depth produces `VoiceError::Backend`.
- An empty reference audio returns `VoiceError::Backend`.
- The unavailable runtime produces `VoiceError::Unavailable` naming the
  acoustic clone tier as the working alternative.

## Testing

Unit tests in `qwen3.rs` verify that:

- loading on an empty directory returns `VoiceError::Unavailable` naming the
  download command;
- a missing decoder is reported separately from a missing talker;
- a file that is not a valid GGUF container is refused rather than accepted;
- the speaker descriptor is a finite 256-dimensional vector.

## Known limits

- **The neural runtime does not exist yet.** Implementing it requires the
  custom `qwen3-tts` architecture (Res2Net speaker encoder + Qwen3 talker
  with mRoPE + code predictor) and the 12 Hz codec decoder in Rust. No
  upstream Rust implementation exists and `candle-transformers` has no
  `qwen3-tts` model.
- The engine therefore validates but does not execute.
- The model manifest points at the Apache-2.0 community GGUF export
  (`Serveurperso/Qwen3-TTS-GGUF`) plus the text-side tokenizer files from
  `Qwen/Qwen3-TTS-12Hz-0.6B-Base`; every size and digest in the manifest was
  read back from the upstream LFS pointers.
- `Qwen3Engine::extract_embedding` currently returns the acoustic
  voiceprint, not a neural speaker embedding. It is format-compatible with
  the acoustic tier's embedding slot but is not what a neural runtime would
  consume.
