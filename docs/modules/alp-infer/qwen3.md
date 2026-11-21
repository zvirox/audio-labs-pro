# `alp-infer` / `qwen3`

## Purpose

The Qwen3-TTS 0.6B neural voice-cloning engine (Apache-2.0, Qwen / Alibaba
Cloud), executed by `crane-core` 26.9.0's Candle implementation inside the
out-of-process `alp-infer` worker (ADRs 0008, 0018, 0029).

## Public API

- `Qwen3Engine::load(model_dir) -> Result<Self, VoiceError>`: verifies the
  official asset layout and constructs the CPU/F32 model.
- `Qwen3Engine::model_dir()`, `talker_path()`, `decoder_path()`: installation
  paths for the model and nested 12 Hz speech-tokenizer weights.
- `TtsBackend` implementation (`id = "qwen3-tts"`): advertises a dynamic clone
  voice. Plain `synthesize` refuses because the Base model requires reference
  conditioning.
- `CloneBackend` implementation:
  - `create_profile(reference, transcript)`: creates a cloned profile with the
    `reference.wav` bundle path and required transcript. It never fabricates
    consent or a neural embedding.
  - `synthesize_with(profile, request)`: performs ICL voice cloning from the
    profile's actual reference recording and transcript.
- Constants: `MODEL_ID = "qwen3-tts"`, `SAMPLE_RATE = 24_000`.

## Data flow

1. **Install.** The manifest identifies seven official Safetensors/config/
  tokenizer files from `Qwen/Qwen3-TTS-12Hz-0.6B-Base`. Missing files produce
  an actionable `VoiceError::Unavailable` naming `alp-cli model download
  qwen3-tts`.
2. **Load.** Crane constructs the model using `Device::Cpu` and `DType::F32`.
  The loaded `Mutex<Model>` serializes mutable generation and remains in the
  daemon's clone slot. Plain-TTS and STT slots are untouched; status reports
  `active_clone_model = qwen3-tts`.
3. **Prepare the profile.** The client resolves a relative bundled reference
  against the profile-store directory on its cloned outbound request. Stored
  profiles stay portable. At render time the worker decodes the reference,
  resamples it to the speaker-encoder rate, and downmixes it to mono.
4. **Generate.** `generate_voice_clone` performs ICL with temperature 0.9,
  `top_p = 1.0`, repetition penalty 1.05, and a request-scaled codec-token
  limit. The acoustic 256-value voiceprint is not used as Qwen conditioning.
5. **Validate and return.** Output must be non-empty, finite, and non-silent;
  the engine returns mono 24 kHz `PcmBuffer`. The daemon enforces consent before
  dispatch. If Qwen is requested but not loaded or generation fails, the request
  errors instead of silently substituting acoustic synthesis.

## Allowed dependencies

- `alp-media`: `PcmBuffer`, `decode`, `resample`
- `alp-voice`: backend traits, profile/request/error types
- `crane-core`: official Qwen3-TTS Candle model, generation options, device and
  tensor types
- Standard-library paths and `Mutex`
- Safe Rust only (`unsafe_code = "deny"`).

## Threading / real-time notes

Daemon-worker process only, never on the real-time audio callback. Model load
and generation are blocking and can be slow on CPU. The mutex allows the engine
to satisfy the thread-safe backend object contract while serializing mutable
inference state.

## Error handling

- Missing weights, tokenizer, or configuration produce `VoiceError::Unavailable`
  with the missing path and download command.
- Invalid assets, model/tensor failures, empty input, missing reference or
  transcript, empty/non-finite/silent output produce `VoiceError::Backend`.
- Plain synthesis without a cloned profile is refused.
- An unloaded Qwen model is refused at the daemon boundary; API, CLI, and GUI
  also prevent a Qwen failure from falling back to the acoustic tier.

## Testing

Unit tests cover missing and corrupt assets, reference downmix/resampling, and
short/long generation limits. Explicit model-gated tests use `ALP_QWEN3_DIR` and
`ALP_QWEN3_REFERENCE` to execute real neural generation without downloading
assets during ordinary test runs. The `clone_daemon` integration test also
checks real socket inference, consent refusal, clone status, and independent
plain-TTS routing; `ALP_QWEN3_OUTPUT` optionally writes its evidence WAV.

## Known limits

- Only Qwen3-TTS 0.6B Base is wired; 1.7B, VoxCPM2, and OmniVoice are separate
  backends/tasks.
- CPU/debug generation is slow and this is not a perceptual speaker-similarity
  quality evaluation.
- ICL requires the reference transcript. The separate acoustic clone tier
  remains available as `acoustic-clone`, but is never presented as Qwen output.
