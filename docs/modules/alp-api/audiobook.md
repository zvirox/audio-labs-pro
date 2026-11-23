# `alp-api` / `audiobook`

## Purpose

Audiobook encode for `ApiSession` (target-A parity: the installed voice-cloning
app's `audiobook` capability). An audiobook script is JSON — either
`{"segments": [...]}` or a bare array of segments, where each segment is
`{ "text", "voice_id", "speed"?, "gap_secs"? }`, the same contract as
`script_voice_ops::tts_script_generate`. The encode synthesizes every segment
with its voice, places one clip per segment on a dedicated `Audiobook` track,
and exports the master with the audiobook delivery preset
(`alp_media::ExportPreset::Audiobook`).

A cancelled encode stops between segments: the job stays `Cancelled` and the
already-placed clips remain on the timeline.

## Public API

- `AudiobookSegment`: `{ text, voice_id, speed?, gap_secs? }` (default gap
  `DEFAULT_SEGMENT_GAP_SECS = 0.35`).
- `AudiobookScript`: untagged enum — `Wrapped { segments }` or
  `Bare(Vec<AudiobookSegment>)`.
- `AudiobookEncodeReport`: `{ track_id, master_path, segment_count,
  total_duration_secs, job_id }`.
- `ApiSession::audiobook_encode(script_json, output_path?)` (Export tier):
  parses the script, validates non-empty texts, known voices and non-negative
  gaps, submits one `JobManager` job, places the clips, exports the master,
  audits the call, and returns the report with the job id set.

## Data flow

1. Tier + JSON parse via `serde_json::from_str::<AudiobookScript>`; validation
   errors name the segment (empty text, negative gap).
2. Voice existence is checked against `ProfileStore::get_profile` before any
   clip is placed (`voice_store()` is now `pub(crate)` for this).
3. `add_track("Audiobook")` — a dedicated track for the narration.
4. Per segment: `tts_generate_inner` (daemon-first synthesis) →
   `media_import` → `clip_add(track, source, cursor, …)`; `jobs.update_progress`;
   the cursor advances by the clip duration plus the inter-segment gap.
5. Master export via `session.export_audio(ExportPreset::Audiobook, path)`
   (default target: `<project>/renders/audiobook-<ms>.wav`; `output_path` is
   honored inside the sandbox).
6. Audit (`audiobook_encode`) and return; the wrapper marks the job
   `Completed` (or `Failed`, or leaves an explicit `Cancelled`).

## Allowed dependencies

- `alp-api` (`ApiSession`; job manager and paths via `self.jobs`/`self.paths`)
- `alp-core` (`now_ms`, `ProjectPaths::renders_dir`)
- `alp-media` (`ExportPreset`, `export_audio`)
- `alp-voice` (`ProfileStore` via `voice_store`)
- `serde`, `serde_json`

## Threading / real-time notes

Synchronous, off the audio thread; per-segment synthesis is the same
daemon-first path as `tts_generate`. The loop checks `jobs.is_cancelled` before
each segment, so `job_cancel` stops the encode between segments.

## Error handling

- `ApiError::Validation` for malformed JSON, empty scripts, empty segment text,
  negative gaps — the job is failed and the call returns the error.
- `ApiError::NotFound` for unknown voices — validated up-front, before any
  placement.
- The job wrapper leaves an explicit `Cancelled` state untouched on the cancel
  path; other failures mark the job failed.

## Testing

`mod tests` cover: encode places clips and exports a real master (WAV file,
non-empty bytes, `job_status` = `completed`), unknown voice rejected
(`NotFound`), empty wrapped and bare scripts rejected (`Validation`).

## Known limits

- Segment-level granularity only: no word-level transcript timing (the
  `edit_remove_fillers` segment-driven path remains the closest tool).
- `total_duration_secs` is the placed-timeline cursor (duration + trailing
  gaps), not the exported file's duration.
- A cancelled encode leaves the already-placed clips on the timeline (no
  rollback); a re-encode adds another `Audiobook` track.
