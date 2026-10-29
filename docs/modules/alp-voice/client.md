# `alp-voice` / `client`

## Purpose

Provides `InferClient` for communicating with the out-of-process `alp-infer` daemon over Unix domain sockets, and `BuiltinAcousticBackend` for instant zero-dependency in-process speech synthesis.

## Public API

- `InferClient`: `new`, `with_socket`, `is_alive`, `ping`, `status`, `load_model`, `synthesize`
- `BuiltinAcousticBackend`: `new`, `with_sample_rate`, implements `TtsBackend`

## Data flow

1. `InferClient::synthesize` connects to Unix socket, sends `InferRequest::Synthesize`, and reads `InferResponse::Audio`.
2. `InferClient::status` / `load_model` exchange `InferRequest::GetStatus` / `LoadModel` for `InferResponse::Status` (`active_model`, `ready`); a `ready: false` load means the daemon declined and kept its previous engine.
3. `BuiltinAcousticBackend` normalizes input text with `TextNormalizer`, resolves vowel formants and consonant noise profiles, and synthesizes 48 kHz PCM audio with natural amplitude envelopes and word pauses.

## Allowed dependencies

- `alp-core::CoreError`, `alp-media::PcmBuffer`
- `crate::protocol`, `crate::text::TextNormalizer`
- Standard library networking (`std::os::unix::net::UnixStream`)

## Threading / real-time notes

`InferClient` and `BuiltinAcousticBackend` run on worker threads or offline CLI contexts. They must not be called from the real-time audio callback.

## Error handling

Returns `VoiceError::Unavailable` if socket connection fails, or `VoiceError::Backend` on synthesis errors.

## Testing

Unit tests in `client.rs` verify that `BuiltinAcousticBackend` synthesizes non-silent audio with valid sample rates and frame counts.

## Known limits

- In-process built-in acoustic model uses rule-based formant resonance; neural Kokoro/Kitten models offer higher prosodic naturalness when downloaded.
