# `alp-infer`

## Purpose

The out-of-process inference worker for Audio Labs Pro. Listens on a Unix domain socket, serves speech synthesis requests, and isolates neural model memory, ONNX runtimes, and compute failures from the UI and audio playback processes.

## Public API

Binary executable with command-line interface:
- `alp-infer [--socket <PATH>] [--once] [--help]`

## Data flow

1. Binds to `$XDG_RUNTIME_DIR/alp-infer.sock` (or specified socket path).
2. Reads framed `InferRequest` messages (`Ping`, `GetStatus`, `LoadModel`, `Synthesize`, `Cancel`, `Shutdown`).
3. Executes speech synthesis via `BuiltinAcousticBackend` or neural backends.
4. Streams progress updates and returns synthesized audio frames as `InferResponse::Audio`.

## Allowed dependencies

- `alp-core`, `alp-media`, `alp-voice`
- `serde`, `serde_json`, `thiserror`
- Standard library networking and filesystem

## Threading / real-time notes

Inference is CPU/worker-bound within this isolated process. It never interacts with the audio callback thread or audio devices.

## Error handling

Request-level failures return typed `InferResponse::Error` payloads without crashing the daemon. Clean socket cleanup occurs on shutdown.

## Testing

Acceptance tested in `scripts/phase6-demo.sh` via socket connection, ping/pong health checks, and audio generation.

## Known limits

- Single-worker event loop; multi-worker pool scaling will be introduced for heavy batch workloads.
