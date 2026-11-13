# `alp-api` / `script_voice_ops`

## Purpose

Turns a JSON script into timeline audio and regenerates a single clip's audio in
place. Both paths synthesize real speech, import the result, and group their
commands so the whole operation is one undo step.

## Public API

- `ScriptRegenerateReport` — `clip_id`, `new_clip_id`, `new_text`, `engine`,
  `duration_secs`, `start_secs`, `single_undo_step`.
- `ScriptLine` — `text`, `voice_id`, `speed?`, `gap_secs?`.
- `ScriptGenerateReport` — `track_id`, `clip_ids`, `engines`,
  `total_duration_secs`, `end_secs`, `single_undo_step`, `job_id`.
- `ApiSession::tts_regenerate(&mut self, clip_id: &str, new_text: &str, voice_id: &str, speed: Option<f32>) -> Result<ScriptRegenerateReport, ApiError>`
- `ApiSession::tts_script_generate(&mut self, script_json: &str) -> Result<ScriptGenerateReport, ApiError>`

The script document is either `{"segments": [ScriptLine, ...]}` or a bare array
of `ScriptLine`.

## Data flow

`tts_regenerate`:

1. Check `PermissionTier::Edit`, record `undo_len`, and look up the clip's
   track and start time.
2. `tts_generate(new_text, voice_id, speed)` writes a WAV into `media/` and
   returns the engine that ran.
3. `media_import` registers the WAV as a source; `clip_remove` removes the old
   clip; `clip_add` places the new source at the original start.
4. `group_commands_since("Regenerate clip", undo_before)` collapses the remove +
   add (+ import) into one undo entry. Neighbouring clips keep their positions.

`tts_script_generate`:

1. Check `PermissionTier::Edit` and submit a `JobManager` job.
2. Parse the document (`segments` object or bare array) and validate each line
   has non-empty `text` and `voice_id`.
3. Synthesize each line, import it, and lay clips out on a track named `Script`
   (created on demand), advancing the cursor by the clip duration plus
   `gap_secs` (default 0.35 s).
4. Group every command since the start into one undo entry and complete the job.
   On error the job is failed with the error message and no clips remain.

## Allowed dependencies

- `serde::{Deserialize, Serialize}`
- `serde_json` (script parsing)
- `crate::error::ApiError`
- `crate::session::ApiSession`
- `crate::types::PermissionTier`
- Synthesis/import via `ApiSession::tts_generate` and `ApiSession::media_import`

## Threading / real-time notes

Synthesis runs through the daemon-first voice path and may block on the
`alp-infer` daemon; the job record makes the work observable while it runs. No
part of this module touches the real-time audio callback.

## Error handling

- `ApiError::Validation` for malformed JSON, an empty script, an empty line
  text or a missing `voice_id`; the message names the offending line index.
- `ApiError::Validation` if `clip_id` is unknown in `tts_regenerate`.
- Synthesis/import errors propagate from the underlying operations, and the job
  is failed before returning.

## Testing

Integration tests in `crates/alp-api/tests/export_and_script.rs`:

- `a_script_becomes_clips_on_its_own_track_in_one_undo_step` — clips land on a
  `Script` track and one undo removes them all.
- `regenerating_a_clip_replaces_its_audio_in_place` — a new clip id at the same
  start, no duplicate left behind.

MCP tests exercise `tts_generate` and the report's `job_id` through
`job_status`.

## Known limits

- Synthesis is sequential; lines are not batched to the daemon in parallel.
- `speed` is passed through where the engine supports it; the built-in
  fallback ignores it.
- The generated track name is fixed (`Script`); callers cannot choose it
  through this API.
- Audio quality depends on whichever engine runs; `engines` names the real one
  per line so a caller can tell daemon output from the built-in fallback.
