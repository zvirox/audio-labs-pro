# `alp-api` / `export_ops`

## Purpose

Lists delivery presets, exports per-track stems, writes real transcripts, and
renders short preview slices. Each export produces the artefact it names; the
stem path is per-track, the transcript path contains real words, and the preview
path is a real WAV.

## Public API

- `PresetListReport` — `presets: Vec<PresetEntry>`.
- `PresetEntry` — `id`, `label`, `description`, `sample_rate`, `bit_depth`.
- `StemExportReport` — `track_id`, `track_name`, `output_path`,
  `duration_secs`, `peak_dbfs`.
- `TranscriptExportReport` — `format`, `path`, `duration_secs`, `engine`,
  `segments`, `text`.
- `PreviewRenderResult` — `start_secs`, `duration_secs`, `sample_rate`,
  `channels`, `peak`, `preview_path`.
- `ApiSession::export_presets(&self) -> Result<PresetListReport, ApiError>`
- `ApiSession::export_stems(&mut self, output_dir: &str, preset_id: &str) -> Result<Vec<StemExportReport>, ApiError>`
- `ApiSession::export_transcript(&mut self, format: &str, path: &str) -> Result<TranscriptExportReport, ApiError>`
- `ApiSession::preview_render(&mut self, start_secs: f64, duration_secs: f64) -> Result<PreviewRenderResult, ApiError>`
- `project_has_synthetic_sources(&self) -> bool` — crate-internal; scans source
  bytes for the synthetic-voice marker.

Presets available: `podcast` (44.1 kHz FLAC), `audiobook` (44.1 kHz WAV),
`voice_over` (48 kHz 24-bit WAV).

## Data flow

1. Check the tier: `Observe` for presets and previews, `Export` for stems and
   transcripts.
2. Validate the output path through `WorkspaceSandbox::validate_path`.
3. Render:
   - **Stems**: for each track, `render_track_mixdown(track_id)` renders that
     track **in isolation**; the file is `stem_NN_<sanitised name>.wav` and the
     report carries that stem's own `peak_dbfs`.
   - **Transcript**: render the project, run the real transcription path
     (daemon STT when available, built-in segmenter otherwise), and write
     `text` / `srt` / `vtt` from `Transcript::to_plain_text` / `to_srt` /
     `to_vtt`. The report echoes the written text.
   - **Preview**: slice the rendered mix at `start_secs` for `duration_secs`,
     write a WAV into `media/`, and report that slice's own peak and length.
4. If the project contains any synthetic source, set
   `metadata.synthetic = true` and a watermark before writing, so the output is
   tagged.
5. Return the typed report and record an audit entry.

## Allowed dependencies

- `serde::{Deserialize, Serialize}`
- `alp_media::{export, ExportPreset, PcmBuffer}`
- `crate::error::ApiError`
- `crate::session::ApiSession`
- `crate::types::PermissionTier`

## Threading / real-time notes

Exports are synchronous offline operations on the caller's thread; they render
through the same engine pipeline as playback but never on the audio callback.
There is no background thread pool, so a large stem export blocks the caller.

## Error handling

- `ApiError::PermissionDenied` below the required tier.
- `ApiError::Validation` for sandbox escapes, an empty project, an unknown
  transcript format, or a preview range past the end of the project.
- `ApiError::Audio` / IO errors from decode and `alp_media::export`.

## Testing

Integration tests in `crates/alp-api/tests/export_and_script.rs`:

- `stems_carry_their_own_track_not_the_master` — a 20 dB track difference
  survives and the stems differ sample-by-sample.
- `transcript_export_contains_real_words_and_timestamps` — the SRT content
  matches the report.
- `preview_render_writes_the_requested_slice` — the written WAV has the
  requested length.

## Known limits

- Exports are synchronous; they are not submitted as `JobManager` jobs (only
  TTS, STT and script generation are).
- `export_stems` renders tracks sequentially.
- `export_transcript` inherits the STT limitation: with the built-in segmenter
  the words are `Speech segment N` placeholders and `engine` says `builtin`.
- There is no AAC/Opus output; presets map to the WAV/FLAC encoders `alp-media`
  provides.
