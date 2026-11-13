# `alp-api` / `smart_edit_ops`

## Purpose

Performs measured "smart" edits on a track: cut silence, shorten long pauses,
remove filler words, and match project loudness to a target. Each operation
reads the rendered track, edits the real timeline with ripple-delete or
gain commands, re-renders, and reports the measured before/after.

## Public API

Reports (all `Serialize`/`Deserialize`):

- `SilenceTrimReport` — `silence_trimmed_secs`, `ranges_cut`, `before_secs`,
  `after_secs`, `threshold_dbfs`, `min_silence_secs`.
- `PauseShortenReport` — `pauses_processed`, `seconds_removed`, `before_secs`,
  `after_secs`, `max_pause_secs`.
- `FillerRemoveReport` — `fillers_removed`, `fillers_found`, `before_secs`,
  `after_secs`, `engine`.
- `LoudnessMatchReport` — `target_lufs`, `before_lufs`, `actual_lufs`,
  `peak_dbfs`, `gain_db`, `limited_by_peak`.

Operations on `ApiSession`:

- `edit_trim_silence(&mut self, track_id: &str, threshold_db: f32, min_silence_secs: f64) -> Result<SilenceTrimReport, ApiError>`
- `edit_shorten_pauses(&mut self, track_id: &str, max_pause_secs: f64, crossfade_ms: u64) -> Result<PauseShortenReport, ApiError>`
- `edit_remove_fillers(&mut self, track_id: &str, filler_word_ids: Option<Vec<String>>) -> Result<FillerRemoveReport, ApiError>`
- `edit_match_loudness(&mut self, target_lufs: f32) -> Result<LoudnessMatchReport, ApiError>`

## Data flow

1. Check `PermissionTier::Edit` and validate arguments (threshold in
   `-96..=0` dBFS, non-negative durations, target in `-70..=0` LUFS).
2. Render:
   - `edit_trim_silence` / `edit_shorten_pauses` call
     `render_track_mixdown(track_id)` and walk the per-frame peak to find silent
     runs.
   - `edit_match_loudness` renders the whole project and measures LUFS.
3. Edit:
   - Trim builds `FrameRange`s for each silent run (leading, interior,
     trailing) and applies `DeleteRange { ripple: true }` commands through
     `execute_command`, latest range first so earlier positions stay valid.
   - Pause shortening keeps `max_pause_secs` of each long pause and cuts the
     excess from the pause's end, preserving the next onset.
   - Loudness matching computes `target - before`, writes each track's new gain
     with `SetTrackGain`, re-measures, and reduces the gain if the sample peak
     would exceed the −1 dBFS ceiling (`limited_by_peak`).
4. Re-render / re-measure and return the report.
5. Refuse safely: `MAX_CUT_RANGES` (512) caps how many ranges one call may cut;
   a request that would exceed it fails with a suggestion to work in passes.

## Allowed dependencies

- `serde::{Deserialize, Serialize}`
- `alp_core::{DeleteRange, FrameRange, SetTrackGain, TrackId, secs_to_frames}`
- `crate::error::ApiError`
- `crate::session::ApiSession`
- `crate::types::PermissionTier`

## Threading / real-time notes

Edits are synchronous on the caller's thread and only touch project state and
offline renders. The playback engine sees the resulting command stack on its
next frame; no work happens on the audio callback.

## Error handling

- `ApiError::Validation` for out-of-range thresholds/targets, negative
  durations, or more than `MAX_CUT_RANGES` ranges.
- `ApiError::Validation` from `edit_match_loudness` when the project renders to
  digital silence (matching loudness on silence is meaningless).
- `ApiError::NotFound` when the track id does not exist.
- `edit_remove_fillers` returns a validation error explaining that word-level
  timestamps are required when the transcript carries only placeholder words.

## Testing

`smart_edit_ops.rs` unit tests:

- `trim_silence_actually_cuts_and_reports_the_new_length`
- `shorten_pauses_keeps_the_allowed_pause_and_cuts_the_excess`
- `filler_removal_refuses_when_only_placeholder_words_exist`
- `match_loudness_applies_real_gain_and_respects_the_peak_ceiling`

MCP end-to-end: `trim_silence_through_mcp_actually_shortens_the_timeline` and
`trim_silence_rejects_an_impossible_threshold`.

## Known limits

- Filler removal depends on word-level timestamps, which no current STT path
  fills. With the built-in segmenter it **refuses** instead of cutting at
  invented positions. Default word list: `um, uh, er, uhm, hmm, mmm`.
- `crossfade_ms` on `edit_shorten_pauses` is accepted but the ripple-delete
  command supplies the crossfade; the value is not applied independently yet.
- Pause shortening works on the silence between onsets, not on a VAD model, so
  quiet-but-present audio can be treated as a pause if it stays under the
  detector's floor.
- Loudness matching scales whole-track gain; it does not build per-clip or
  per-section automation.
