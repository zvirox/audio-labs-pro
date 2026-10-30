# `alp-infer`

## Purpose

The out-of-process inference worker for Audio Labs Pro. Listens on a Unix domain socket, serves speech synthesis and transcription requests, and isolates neural model memory, ONNX runtimes, and compute failures from the UI and audio playback processes.

## Public API

Binary executable with command-line interface:
- `alp-infer [--socket <PATH>] [--once] [--help]`

## Data flow

1. Binds to `$XDG_RUNTIME_DIR/alp-infer.sock` (or specified socket path).
2. Reads framed `InferRequest` messages (`Ping`, `GetStatus`, `LoadModel`, `Synthesize`, `Transcribe`, `DesignVoice`, `Cancel`, `Shutdown`).
3. Executes speech synthesis via the builtin acoustic tier or a loaded neural `style_text_to_speech_2` engine: Kokoro-82M (`kokoro.rs`) or Kitten Nano (`kitten.rs`), both over the shared layer in `style_tts2.rs` (ONNX Runtime via `ort`, MIT G2P front-end from `piper-plus-g2p` with the bundled CMUdict, sentence-chunked long-text reconstruction per ADRs 0023/0024).
4. Executes transcription via the builtin acoustic STT tier or the loaded neural engine: Moonshine Tiny (`moonshine.rs`, ADR 0025 - 16 kHz encoder, merged decoder with KV cache, Llama-style BPE out).
5. `LoadModel` dispatches by tier: `moonshine-tiny` loads into the transcription slot without touching synthesis; `kokoro-82m` and `kitten-nano` load into the synthesis slot; anything else is declined honestly. A failed load keeps the previously active engine of that tier and answers `ready: false`. The `builtin-acoustic` reset restores the synthesis tier only - the built-in STT is shadowed, never unloaded.
6. `Status` reports both slots (`active_model`, `ready`, `active_stt_model`); progress updates stream for synthesis/transcription work and results return as `InferResponse::Audio` (24 kHz mono for both neural TTS engines) or `InferResponse::Transcript` (segments in the caller's frame space).

## Allowed dependencies

- `alp-core`, `alp-media`, `alp-voice`
- `ort` (ONNX Runtime bindings, pinned per ADR 0008), `piper-plus-g2p` (G2P, per ADR 0010)
- `serde`, `serde_json`, `thiserror`
- Standard library networking and filesystem

## Threading / real-time notes

Inference is CPU/worker-bound within this isolated process. It never interacts with the audio callback thread or audio devices. Clients are served sequentially on the accept loop, so loaded models are constructed once per daemon and reused across connections; each engine's ONNX session sits behind a mutex and is idle between requests. All neural engines share the single process-wide `ort::Environment` (rc.15 allows one live environment per process; ADR 0025).

## Error handling

Request-level failures return typed `InferResponse::Error` payloads without crashing the daemon. Clean socket cleanup occurs on shutdown.

## Testing

Acceptance tested in `scripts/phase6-demo.sh` via socket connection, ping/pong health checks, and audio generation. Engine unit tests pin the load contracts (missing files decline honestly, contents decline at the parse/graph stage, corrupt graphs keep the previous engine); the two-slot test loads kitten and Moonshine together and asserts independent reset; gated real-model smoke tests transcribe kitten-spoken speech through Moonshine (ADR 0025).

## Known limits

- Single-worker event loop; multi-worker pool scaling will be introduced for heavy batch workloads.
- Neural transcription is non-streaming and capped at 300 s per call (ADR 0025).
