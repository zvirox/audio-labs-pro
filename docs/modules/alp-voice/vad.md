# `alp-voice` / `vad`

## Purpose

Provides streaming and batch Voice Activity Detection (VAD) using adaptive energy thresholding, zero-crossing rate analysis, hangover suppression, and speech duration debouncing to segment continuous audio into discrete speech intervals.

## Public API

- `VadConfig`: `threshold_db`, `hangover_ms`, `min_speech_ms`, `frame_size_ms`
- `VoiceActivityDetector`: `new`, `process_block`, `is_speech`, `reset`, `config`, `sample_rate`, `consecutive_speech_frames`
- `segment_speech`: Segment a complete `PcmBuffer` into discrete speech `FrameRange` intervals.

## Data flow

1. Input audio is analyzed frame-by-frame (typically 20 ms blocks) measuring RMS signal energy and zero-crossing rates.
2. Signal bursts above the dBFS energy threshold within the voice frequency zero-crossing band trigger speech accumulation.
3. Once accumulated frames exceed `min_speech_ms`, speech state is confirmed and onset is backdated to the initial burst.
4. When signal drops below threshold, hangover frames prevent premature segment chopping during inter-word pauses.
5. Returns contiguous `FrameRange` intervals representing speech utterances.

## Allowed dependencies

- `alp-core::FrameRange`
- `alp-media::PcmBuffer`
- `serde`, Standard library

## Threading / real-time notes

`VoiceActivityDetector::process_block` is real-time safe: zero heap allocations, zero locks, deterministic $O(N)$ execution over the frame buffer.

## Error handling

VAD methods accept valid audio slices and return boolean speech states or a vector of non-empty frame ranges. Invalid sample bounds are clipped gracefully.

## Testing

Unit tests in `vad.rs` verify synthetic tone burst detection surrounded by silence, verifying that speech onset and offset timestamps fall within expected millisecond bounds.

## Known limits

- Time-domain energy and ZCR based; deep learning ONNX models (e.g. Silero VAD) can be added as an optional download model.
