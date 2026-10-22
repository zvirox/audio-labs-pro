# `alp-infer`

## Purpose

The out-of-process inference worker. Keeping ONNX Runtime and GGUF model
loading in a separate process means an out-of-memory condition or GPU driver
crash cannot take down the editor or the audio stream. Phase 1 establishes the
binary and the process boundary; models arrive in Phase 6.

## Public API

The crate is a binary with no library API. Phase 1 `main` prints a notice and
exits successfully so packaging and CI can build it.

## Data flow

Planned: the worker reads requests from a versioned Unix socket, runs TTS, STT
or cloning, and returns audio or transcripts. `alp-voice` owns the protocol
types. The worker never reads project files or touches the audio device.

## Allowed dependencies

`alp-core`, `alp-media` and `alp-voice` are allowed by the architecture. No
other ALP crate may depend on `alp-infer`; it is reached only at runtime over
the socket.

## Threading / real-time notes

Model inference and audio model I/O run on worker threads inside this process.
The real-time audio callback never calls into inference.

## Error handling

Phase 1 exits successfully after printing a notice. The future protocol will
return typed error responses and keep the process alive after a failed request.

## Testing

CI builds the binary. Backend and protocol tests arrive with the runtime in
Phase 6.

## Known limits

- No model runtime, socket server or manifest loading yet.
- No GPU selection, memory limits or crash-recovery policy yet.
