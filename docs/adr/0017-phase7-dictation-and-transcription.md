# 0017 — Dictation, Speech Activity Detection, and Text-Based Editing

- Status: accepted
- Date: 2026-10-27
- Phase: 7 (Dictation and transcription)

## Context

Phase 7 brings bidirectional speech capabilities to Audio Labs Pro by enabling Speech-to-Text (STT), streaming microphone dictation, SubRip / WebVTT subtitle export, and text-based timeline editing. Podcasters, narrators, and audio creators require fast transcription with sub-second feedback during live recording, accurate speech segmentation to remove silences, and direct linkage between edited text and timeline audio frame ranges (ripple cutting audio by simply deleting text segments).

## Decision

### Voice Activity Detection (VAD)

1. Implemented `VoiceActivityDetector` in `alp-voice` using dual-criterion energy RMS thresholding (configurable dBFS) and zero-crossing rate (ZCR) windowing.
2. Added hangover frame buffering to avoid clipping speech trailing off during natural vocal pauses.
3. Added speech onset backdating based on minimum speech duration debouncing to ensure initial word transients are not cut.
4. Implemented `segment_speech(&PcmBuffer, &VadConfig) -> Vec<FrameRange>` to partition long recordings into discrete speech intervals.

### Streaming Dictation and STT Backends

1. Implemented streaming `DictationSession` emitting `DictationEvent` (`SpeechStarted`, `Partial`, `Final`, `Silence`) for live microphone input.
2. Implemented `BuiltinSttBackend` providing baseline acoustic energy/duration transcription without requiring external neural model weights.
3. Extended `alp-infer` IPC protocol with `InferRequest::Transcribe` and `InferResponse::Transcript`, enabling out-of-process neural transcription (Moonshine Tiny) while falling back seamlessly to built-in STT if the worker is offline.

### Transcript Document Model and Subtitle Formats

1. Implemented `Transcript` owning an ordered sequence of `TranscriptSegment` objects with frame-accurate start and end timestamps.
2. Implemented export to plain text (`to_plain_text`), standard SubRip subtitle format (`to_srt` using comma millisecond separators: `00:00:01,234`), and WebVTT (`to_vtt` with `WEBVTT` header and period millisecond separators: `00:00:01.234`).
3. Implemented standard SRT parser (`parse_srt`) converting subtitle files back into frame-accurate transcript documents.

### Text-Based Timeline Editing

1. Implemented `Transcript::cut_range_for_segment(index) -> Option<FrameRange>`.
2. Wired `EditSession::delete_range(range, ripple)` and UI `Message::DeleteTranscriptSegment(index)` in `alp-ui`, allowing users to delete a transcript line and automatically execute an undoable ripple deletion of that audio interval from the timeline.
3. Added click-to-seek (`Message::SelectTranscriptSegment(index)`) seeking the timeline playhead directly to the spoken phrase.

### CLI and UI Integration

1. Implemented `alp-cli transcribe` and `alp-cli vad` with text, SRT, VTT, and JSON output formats.
2. Integrated Voice & Dictation Lab in `alp-ui` with real-time dictation preview, live toggle, transcript browsing, click-to-seek, and one-click ripple cutting.

## Consequences

- Live dictation perceived latency is sub-second (< 100 ms to partial progress).
- Podcasters can export standard SRT/VTT subtitles directly from recorded episodes.
- Text-based editing allows rapid rough-cut trimming directly from transcript reading.
