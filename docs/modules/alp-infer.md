# `alp-infer`

## Purpose

The out-of-process inference worker for Audio Labs Pro. Listens on a Unix domain socket, serves speech synthesis requests, and isolates neural model memory, ONNX runtimes, and compute failures from the UI and audio playback processes.

## Public API

Binary executable with command-line interface:
- `alp-infer [--socket <PATH>] [--once] [--help]`

## Data flow

1. Binds to `$XDG_RUNTIME_DIR/alp-infer.sock` (or specified socket path).
2. Reads framed `InferRequest` messages (`Ping`, `GetStatus`, `LoadModel`, `Synthesize`, `Transcribe`, `DesignVoice`, `Cancel`, `Shutdown`).
3. Executes speech synthesis via the builtin acoustic tier or a loaded neural `style_text_to_speech_2` engine: Kokoro-82M (`kokoro.rs`) or Kitten Nano (`kitten.rs`), both over the shared layer in `style_tts2.rs` (ONNX Runtime via `ort`, MIT G2P front-end from `piper-plus-g2p` with the bundled CMUdict, sentence-chunked long-text reconstruction per ADRs 0023/0024).
4. `LoadModel` activates a neural engine only on a successful load (`kokoro-82m` and `kitten-nano` are wired; anything else is declined honestly); a failed load keeps the previously active engine and answers `ready: false` for the requested model.
5. Streams progress updates and returns synthesized audio frames as `InferResponse::Audio` (24 kHz mono for both neural engines).

## Allowed dependencies

- `alp-core`, `alp-media`, `alp-voice`
- `ort` (ONNX Runtime bindings, pinned per ADR 0008), `piper-plus-g2p` (G2P, per ADR 0010)
- `serde`, `serde_json`, `thiserror`
- Standard library networking and filesystem

## Threading / real-time notes

Inference is CPU/worker-bound within this isolated process. It never interacts with the audio callback thread or audio devices. Clients are served sequentially on the accept loop, so the loaded model (a ~326 MB session) is constructed once per daemon and reused across connections; the ONNX session sits behind a mutex and is idle between requests.

## Error handling

Request-level failures return typed `InferResponse::Error` payloads without crashing the daemon. Clean socket cleanup occurs on shutdown.

## Testing

Acceptance tested in `scripts/phase6-demo.sh` via socket connection, ping/pong health checks, and audio generation.

## Known limits

- Single-worker event loop; multi-worker pool scaling will be introduced for heavy batch workloads.
