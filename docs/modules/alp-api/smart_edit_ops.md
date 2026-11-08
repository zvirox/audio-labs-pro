# `alp-api` / `smart_edit_ops`

## Purpose

Provides intelligent voice editing operations (silence trimming, pause shortening, filler word cutting, and target loudness matching) for `ApiSession`.

## Public API

- `SilenceTrimReport`: trimmed silence duration, before and after durations.
- `PauseShortenReport`: count of pauses processed, seconds removed, before and after durations.
- `FillerRemoveReport`: count of fillers removed, before and after durations.
- `LoudnessMatchReport`: target LUFS, measured actual LUFS, true peak level, applied gain in dB.
- `ApiSession::edit_trim_silence(&mut self, track_id: &str, threshold_db: f32, min_silence_secs: f64) -> Result<SilenceTrimReport, ApiError>`
- `ApiSession::edit_shorten_pauses(&mut self, track_id: &str, max_pause_secs: f64, crossfade_ms: u64) -> Result<PauseShortenReport, ApiError>`
- `ApiSession::edit_remove_fillers(&mut self, track_id: &str, filler_word_ids: Option<Vec<String>>) -> Result<FillerRemoveReport, ApiError>`
- `ApiSession::edit_match_loudness(&mut self, target_lufs: f32) -> Result<LoudnessMatchReport, ApiError>`

## Data flow

1. Verifies permission tier (`PermissionTier::Edit`).
2. Renders or analyzes track PCM buffers through `render_project_mixdown`.
3. Detects silence intervals, excessive pauses, or filler words based on speech transcript word timestamps (`WordTimestamp`).
4. Performs non-destructive edits or computes loudness gain corrections.
5. Returns a structured report summarizing the operation results.

## Allowed dependencies

- `serde::{Deserialize, Serialize}`
- `alp_dsp::measure_lufs`
- `crate::error::ApiError`
- `crate::session::ApiSession`
- `crate::types::PermissionTier`
- `crate::voice::transcribe_daemon_first`

## Threading / real-time notes

Smart editing operates offline on worker threads; audio editing buffers are detached from the active playback callback.

## Error handling

Returns `ApiError::NotFound` if the specified track cannot be found; `ApiError::Audio` on DSP or render failures.

## Testing

Unit tests in `smart_edit_ops.rs` verify `edit_remove_fillers` accurately matches target filler words from speech transcripts and reports removed durations.

## Known limits

- Advanced audio wave splicing for filler cutting applies frame-level truncation; fine crossfading can be applied in subsequent timeline passes.
