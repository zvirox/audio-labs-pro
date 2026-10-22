# `alp-voice`

## Purpose

Owns the TTS, STT and voice-cloning traits and the data types exchanged between
the app and the out-of-process inference worker. Phase 1 defines the contract
only; it does not load models or start a socket.

## Public API

- `VoiceError`
- `VoiceInfo`, `VoiceKind`, `VoiceProfile`
- `TtsRequest`, `TranscriptSegment`
- `TtsBackend`, `CloneBackend`, `SttBackend`

## Data flow

The UI calls a backend trait through `alp-voice`. In Phase 6 the implementation
will live in `alp-infer` and communicate over a versioned Unix-socket protocol
owned by this crate. Audio crosses the boundary as `PcmBuffer`.

## Allowed dependencies

`alp-core` and `alp-media` (for `PcmBuffer`). It must not depend on the UI,
engine or inference runtime. `alp-infer` depends on `alp-voice`, never the
reverse.

## Threading / real-time notes

Trait methods may block for model inference and must run on a worker, never on
the audio callback. Backends are `Send`; no method is called from the real-time
thread.

## Error handling

`VoiceError` distinguishes unknown voices, unavailable backends and backend
failures. Protocol and manifest errors will be added with the socket in Phase 6.

## Testing

Phase 1 has no voice tests; the traits and data types are exercised when the
Phase 6 backend and socket protocol arrive. Backend tests will cover manifest
loading, protocol errors and request cancellation.

## Known limits

- No socket protocol, manifest loader or backend implementation yet.
- No streaming synthesis; requests return a complete `PcmBuffer`.
