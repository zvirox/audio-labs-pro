# 0008 — Inference process and heavy-model runtime (D3)

- Status: accepted for the process boundary; heavy runtime explicitly deferred
- Date: 2026-10-22
- Spike: `spikes/model-spike`

## Context

Light models (Kokoro, Kitten, Moonshine) run through ONNX Runtime (`ort`).
Heavy cloning models (Qwen3-TTS 0.6B/1.7B GGUF, VoxCPM2, OmniVoice) are large,
may use GPU/Vulkan/CUDA and can OOM. The plan requires that a crash never takes
down the editor.

## Evidence

- `ort` 2.0.0-rc.15 loads Kokoro-82M (326 MB fp32 ONNX) in 1.1–1.4 s and
  synthesizes 2.25 s of speech in 0.59–0.82 s (RTF 0.26–0.36) on an Intel
  TigerLake CPU, with a MIT G2P front-end (ADR 0010).
- Public Qwen3-TTS GGUF work reports CPU RTF 1.14 (Q4) / 2.66 (F16) and
  GPU/Vulkan RTF ≈0.6 on an i9 + RTX 2080 Ti. The Rust example repository has
  **no license**, so its code cannot be reused; the approach (llama.cpp for the
  LLM + ONNX decoder) is sound and both underlying projects are permissive.
- This machine has no discrete GPU (Intel Iris Xe), so GPU/Vulkan heavy-model
  timings could not be measured here.

## Decision

- `alp-infer` is a **separate process** from day one, speaking a versioned
  Unix-socket protocol; it owns all model memory and is restartable.
- Light tier uses `ort` with CPU execution provider.
- Heavy tier is **deferred to Phase 8** with the runtime choice between
  llama.cpp/ggml FFI and `candle` left open; the decision needs a machine with a
  supported GPU and a measured spike. Default plan: llama.cpp GGUF for the LLM
  and `ort` for the audio decoder, both inside `alp-infer`.
- No Python sidecar: it breaks the "fully Rust" rule and packaging simplicity.

## Consequences

- The app remains fully usable with no heavy model installed.
- `alp-voice` owns the protocol types so UI and CLI never link inference code.
- A heavy-model spike (VRAM, RTF, crash recovery) is a Phase 8 gate.
