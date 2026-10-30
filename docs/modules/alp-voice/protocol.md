# `alp-voice` / `protocol`

## Purpose

Defines the IPC message framing and request/response schema used between the application and the `alp-infer` worker process over Unix domain sockets.

## Public API

- `default_socket_path() -> PathBuf`
- `InferRequest`: `Ping`, `GetStatus`, `LoadModel`, `Synthesize`, `Transcribe`, `DesignVoice`, `Cancel`, `Shutdown`
- `InferResponse`: `Pong`, `Status`, `Progress`, `Audio`, `Transcript`, `VoiceDesigned`, `Error`
- `InferStatus`: the `Status` payload as a struct - `active_model` (synthesis tier), `ready` (last load attempt), `active_stt_model` (transcription tier; `None` while the built-in acoustic backend serves). The two engine slots are independent (ADR 0025)
- `write_message<W: Write, T: Serialize>(writer: &mut W, msg: &T) -> Result<(), CoreError>`
- `read_message<R: Read, T: Deserialize>(reader: &mut R) -> Result<T, CoreError>`

## Data flow

1. Each message is encoded with a 4-byte little-endian length prefix followed by UTF-8 JSON.
2. `write_message` writes and flushes the framed payload to the stream.
3. `read_message` reads the exact length prefix and deserializes the payload.

## Allowed dependencies

- `alp-core::CoreError`
- `crate::TtsRequest`
- `serde`, `serde_json`, Standard library I/O

## Threading / real-time notes

IPC calls run on worker threads or background tasks; never on the audio callback.

## Error handling

Returns `CoreError::Validation` if a message exceeds 64 MB maximum payload limit, and translates I/O or JSON parsing errors.

## Testing

Unit tests in `protocol.rs` verify length-prefixed framing and round-trip serialization of synthesis requests and responses.

## Known limits

- Single Unix stream per connection; multiplexed concurrent requests use request ID tags.
