# `alp-voice` / `transcript`

## Purpose

Provides the core transcript document model, timecode formatting, subtitle export (plain text, SubRip SRT, WebVTT), SRT parsing, and timeline frame range cut mapping for text-based audio editing.

## Public API

- `Transcript`: `new`, `add_segment`, `duration_frames`, `duration_secs`, `to_plain_text`, `to_srt`, `to_vtt`, `parse_srt`, `cut_range_for_segment`
- `TranscriptSegment`: `start_frame`, `end_frame`, `text`

## Data flow

1. Speech-to-text recognizers or VAD engines produce sequences of `TranscriptSegment` objects with frame-accurate boundaries.
2. `Transcript` formats segments into TXT, SRT (`00:00:01,234`), or WebVTT (`00:00:01.234`) formats.
3. SubRip SRT files can be parsed back into `Transcript` objects with sample-rate aware frame calculation.
4. When a user deletes or trims text in the transcript, `cut_range_for_segment` calculates the exact `FrameRange` to execute ripple deletion on the timeline.

## Allowed dependencies

- `alp-core::{CoreError, FrameRange}`
- `alp-voice::TranscriptSegment`
- `serde`, Standard library

## Threading / real-time notes

Transcript formatting, parsing, and export run synchronously on the UI or worker thread and do not touch the audio render callback.

## Error handling

Returns `CoreError::Validation` if SRT subtitle timecode strings fail formatting specifications or contain invalid numbers.

## Testing

Unit tests in `transcript.rs` verify round-trip export and parsing of SRT documents, WebVTT header and millisecond period syntax, and `FrameRange` cut mapping.

## Known limits

- In-memory transcript representation; rich styling and speaker diarization tags can be added in future revisions.
