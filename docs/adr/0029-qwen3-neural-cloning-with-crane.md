# ADR 0029: Execute Qwen3-TTS 0.6B in the Inference Worker with Crane

- **Status:** Accepted
- **Date:** 2026-11-17
- **Deciders:** Audio Labs Pro Engineering
- **Supersedes:** the Qwen3-TTS runtime deferral in ADR 0028 and the heavy-runtime
  implementation choice left open by ADR 0008
- **Preserves:** the acoustic clone tier, profile/consent rules, and inference-process
  boundary from ADRs 0008, 0018, 0025, and 0028

## Context and Problem Statement

ADR 0028 shipped a working acoustic clone tier but deferred neural Qwen3-TTS because
its speaker encoder, talker, code predictor, and 12 Hz speech tokenizer form a custom
architecture. P0.4 requires real neural inference through the existing profile,
consent, IPC, and daemon boundaries. Substituting formant synthesis while labelling it
Qwen output would violate the project's routing and provenance rules.

`crane-core 26.9.0` provides a Candle implementation of the required architecture and
loads the official Safetensors assets directly. The daemon needs to retain the loaded
model across requests while satisfying the thread-safe backend object contract.

## Decision Drivers

- Execute the official Qwen3-TTS 0.6B Base model, not an acoustic approximation.
- Keep clone creation human-consented and clone rendering reference-conditioned.
- Keep neural memory and CPU work inside `alp-infer`.
- Reuse the existing `CloneBackend`, profile request, and model manager.
- Report which clone engine actually rendered each result.
- Avoid a Python sidecar, unsafe FFI, and unpinned inference code.

## Considered Options

1. **Use pinned `crane-core`=26.9.0 and official Safetensors assets.** Reuses a working
   Candle implementation without changing checkpoint formats.
2. **Implement the architecture locally.** Duplicates a substantial runtime and delays
   P0.4 without improving the established process or consent boundaries.
3. **Use a Python sidecar.** Adds a second runtime and packaging burden; previously
   rejected by ADR 0008.
4. **Keep the acoustic-only tier.** Does not satisfy P0.4.

## Decision

1. **Runtime and model.** `Qwen3Engine` pins Crane exactly, loads with `Device::Cpu`
   and `DType::F32`, and calls `generate_voice_clone`. The manifest contains seven
   files from `Qwen/Qwen3-TTS-12Hz-0.6B-Base`: combined model weights, nested speech
   tokenizer weights/configuration, and text tokenizer/configuration assets. Model
   downloads and direct installs create nested parent directories.
2. **Conditioning.** ICL consumes the actual reference recording and transcript. The
   client resolves portable bundle-relative paths only on the outbound request; it
   does not rewrite stored profiles. Reference audio is decoded, resampled to the
   speaker encoder's rate, and downmixed to mono. The acoustic 256-value voiceprint is
   never passed as a neural embedding.
3. **Execution and lifecycle.** `Mutex<Model>` serializes mutable generation state.
   A loaded engine remains in the daemon's independent clone slot. Generation uses
   temperature 0.9, `top_p = 1.0`, repetition penalty 1.05, and request-scaled codec
   token limits. Output must be non-empty, finite, non-silent mono 24 kHz PCM.
4. **Routing and provenance.** `active_clone_model` is separate from `active_model`
   and `active_stt_model`; old status messages default it to `None`. API and CLI
   labels use this clone-specific identity. An unloaded or failed Qwen profile returns
   an error at the daemon boundary and is never silently rendered acoustically. The
   acoustic clone backend remains available under its own profile/backend identity.
5. **Toolchain.** Crane's `hf-hub 1.0.0` -> `hf-xet` -> `xet-runtime 1.8.1` chain
   requires `sysinfo 0.39`; every `sysinfo 0.39.x` release declares Rust 1.95. The
   workspace MSRV therefore moves from 1.92 to 1.95 rather than patching the runtime.
6. **Obsoleted artifacts.** Qwen no longer uses the community GGUF export. The small
   GGUF parser is retained only for its unit tests and is not part of production
   engine loading.

## Validation

- An isolated CPU harness produced finite, non-silent mono 24 kHz `pcm_f32le` audio
  (3.28 seconds, RMS 0.08892459).
- `the_daemon_executes_real_qwen_cloning_and_preserves_consent_when_installed` ran the
  actual worker and production socket protocol. It verified model load, consent
  refusal before attestation, consented neural output, `qwen3-tts` clone status, and
  unchanged `builtin-acoustic` plain-TTS status. It passed in 1398.32 seconds in the
  debug test profile.
- The daemon evidence WAV is 78,720 frames / 3.28 seconds, mono 24 kHz float PCM,
  all finite, peak 0.51757520, RMS 0.08892675.
- The direct `official_model_generates_neural_audio_when_installed` test passed
  against the same official assets and reference in 1537.33 seconds in the debug
  test profile.
- These checks demonstrate genuine execution and routing; they are not a perceptual
  speaker-similarity evaluation.

## Consequences

- P0.4 has a real neural runtime behind the established interfaces.
- Initial CPU/debug inference is slow. Performance work must not replace or relabel
  the requested backend.
- Qwen requires its full official asset set and a reference transcript; missing
  assets, references, transcripts, consent, or model load produce explicit errors.
- Only the 0.6B Base model is wired. Qwen 1.7B, VoxCPM2, and OmniVoice remain outside
  this decision's  implementation scope.
