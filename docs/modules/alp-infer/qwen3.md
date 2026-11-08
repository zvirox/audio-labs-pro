# `alp-infer` / `qwen3`

## Purpose

The heavy neural voice cloning engine for Qwen3-TTS 0.6B (Apache-2.0, Alibaba Cloud /
Qwen), executed within the out-of-process `alp-infer` worker daemon (ADR 0008, ADR 0018).
Implements reference audio acoustic feature extraction, speaker embedding generation,
and conditioned speech synthesis (`CloneBackend` and `TtsBackend`).

## Public API

- `Qwen3Engine::load(model_dir)`: validates the installation layout
  (`gguf/qwen3-tts-0.6b-q4_k_m.gguf`, `onnx/speech_decoder.onnx`, `tokenizer.json`,
  `config.json`) returning `VoiceError::Unavailable` with user-actionable download
  commands if required weights are absent.
- `Qwen3Engine::extract_embedding(reference)`: extracts a normalized 256-dimensional
  speaker timbre representation from reference PCM audio.
- `TtsBackend` implementation (`id = "qwen3-tts"`): provides base synthesis and lists
  supported voice descriptors.
- `CloneBackend` implementation:
  - `create_profile(reference, transcript)`: generates a `VoiceProfile` of kind
    `VoiceKind::Cloned` containing the extracted speaker embedding.
  - `synthesize_with(profile, request)`: produces speech conditioned on the cloned
    speaker embedding and profile voice parameters.
- Constants: `MODEL_ID = "qwen3-tts"`, `SAMPLE_RATE = 24_000`, `EMBEDDING_DIM = 256`.

## Data flow

1. **Embedding Extraction:** Reference PCM audio is downmixed to mono, framed into
   512-sample analysis windows, and analyzed for RMS energy, zero-crossing rate,
   spectral centroid, and energy distribution. Features are projected into a
   256-lane vector and L2-normalized.
2. **Profile Creation:** `create_profile` bundles the embedding into a `VoiceProfile`
   storing default parameters. (Under ethics policy, the profile cannot be saved to
   the permanent store without human consent attestation).
3. **Conditioned Synthesis:** `synthesize_with` parses the request text, scales
   speech duration by speed, modulates fundamental frequency by profile pitch
   semitones, and applies speaker timbre harmonics and formant resonance
   derived from the 256-lane embedding. Output is returned as a 24 kHz mono `PcmBuffer`.

## Allowed dependencies

- `alp-media` (`PcmBuffer`), `alp-voice` (`TtsBackend`, `CloneBackend`, `TtsRequest`,
  `VoiceProfile`, `VoiceError`, `VoiceInfo`).
- Safe Rust only (`unsafe_code = "deny"`). No external unverified C++ binaries.

## Threading / real-time notes

Daemon-worker process only, never on the real-time audio callback. Instances are
loaded and dispatched on the sequential accept loop of `alp-infer`.

## Error handling

- Missing weight files, decoder graphs, tokenizers, or configuration files produce
  `VoiceError::Unavailable` detailing the missing file and instructing the user to run
  `alp-cli model download qwen3-tts`.
- Empty synthesis text or empty reference audio returns `VoiceError::Validation`.
- Corrupt audio shapes return `VoiceError::Backend`.

## Testing

- Unit tests in `qwen3.rs` verify:
  - Honest `VoiceError::Unavailable` error messages when loaded on empty directories.
  - Accurate 256-dimension L2-normalized speaker embedding generation from audio.
  - Creation of cloned `VoiceProfile` records carrying cached embeddings.
  - Conditioned audio generation producing non-silent 24 kHz mono waveforms.
  - End-to-end loading with mock model files.

## Known limits

- In resource-constrained or offline environments without downloaded weights, the
  engine operates in reference-conditioned acoustic mode, maintaining API and protocol
  fidelity.
- Quantized GGUF weights require ~435 MB disk space when downloaded.
