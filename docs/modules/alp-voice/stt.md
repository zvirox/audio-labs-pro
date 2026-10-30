# `alp-voice` / `stt`

## Purpose

Provides speech-to-text recognition backends, lightweight embedded acoustic transcription fallback, and streaming `DictationSession` for live low-latency microphone dictation.

## Public API

- `DictationEvent`: `SpeechStarted`, `Partial(String)`, `Final(TranscriptSegment)`, `Silence`
- `DictationSession`: `new`, `process_input`, `sample_rate`, `vad_config`
- `BuiltinSttBackend`: `new`, `transcribe`, `streaming`

## Data flow

1. In streaming mode, `DictationSession` consumes incoming microphone buffers.
2. Signal energy is analyzed using voice activity detection.
3. During active speech, incremental duration partial events are emitted to provide immediate user feedback (< 100 ms).
4. Once speech pauses past hangover duration, a finalized `TranscriptSegment` is emitted and internal buffers are reset.
5. In batch mode, `BuiltinSttBackend` segments audio through VAD and transcribes detected speech intervals into `TranscriptSegment` entries.

## Allowed dependencies

- `alp-core::FrameRange`
- `alp-media::PcmBuffer`
- `alp-voice::{SttBackend, TranscriptSegment, VadConfig, VoiceError, segment_speech}`
- Standard library

## Threading / real-time notes

`DictationSession::process_input` operates efficiently with fixed linear time over input frames. Buffering accumulates only during speech bursts.

## Error handling

Returns `VoiceError::Backend` or `VoiceError::Unavailable` if backend processing fails or models cannot be loaded.

## Testing

Unit tests in `stt.rs` verify batch transcription segmentation and streaming dictation event sequences (`SpeechStarted` $\rightarrow$ `Partial` $\rightarrow$ `Final`).

## Known limits

- `BuiltinSttBackend` provides an acoustic energy/duration baseline; it is also the daemon-side and in-process fallback whenever a neural transcription model is not loaded. Neural STT (Moonshine Tiny) loads into the `alp-infer` daemon's transcription tier and serves through the same `SttBackend` trait (ADR 0025).
