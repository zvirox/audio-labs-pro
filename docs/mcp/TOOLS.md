# Audio Labs Pro — MCP Tool Reference

All 58 tools exposed by `alp-mcp`. Tools are filtered by the session's
permission tier (`Observe` < `Edit` < `Export`), and every call is audited.

Conventions used below:

- **Handles** are opaque UUID strings returned by the tool that created the
  object (`track_id`, `clip_id`, `source_id`, `job_id`, `checkpoint_id`).
- **Times are seconds** (floats). Sample-accurate frame maths happens inside the
  session; reports always carry `sample_rate` and `channels` where relevant.
- **Every result** carries `structuredContent` plus a one-line text summary.
  Failures set `isError: true` and explain the cause and the next call to make.
- **Everything is undoable.** One tool call is one undo group, except
  `edit_batch`, `tts_script_generate` and `tts_regenerate`, which group their
  whole effect into a single step.
- Behaviour limits are collected in [`AUDIT.md`](AUDIT.md).

---

## 1. Projects & history

| Tool | Tier | Inputs | Result |
|------|------|--------|--------|
| `project_list` | Observe | — | `[ProjectListItem]` — projects found in the workspace |
| `project_info` | Observe | — | `{ id, name, revision, sample_rate, duration_secs, track_count, clip_count, undo_steps, redo_steps, is_dirty }` |
| `project_create` | Edit | `name` | `{ project_id, path }` |
| `project_open` | Edit | `path` | the opened project's summary |
| `project_save` | Edit | — | `{ saved: true }` |
| `project_close` | Edit | — | `{ closed: true }` — reverts to an empty draft |
| `history_list` | Observe | — | `[HistoryEntry]` |
| `history_undo` | Edit | — | `{ undone: bool }` |
| `history_redo` | Edit | — | `{ redone: bool }` |
| `checkpoint_create` | Edit | `label` | `{ checkpoint_id }` |
| `checkpoint_restore` | Edit | `checkpoint_id` | `{ restored: true }` |
| `rollback_session` | Edit | — | `{ rolled_back: bool }` — restores the session-start checkpoint |

`undo_steps`/`redo_steps` in `project_info` are availability flags (0/1), not
counts.

## 2. Media

| Tool | Tier | Inputs | Result |
|------|------|--------|--------|
| `media_list` | Observe | — | `{ media: [{ id, name, relative_path, sample_rate, channels, frames, duration_secs, peak, file_exists }] }` |
| `media_import` | Edit | `path`, `name?` | `{ source_id, name, relative_path, path, duration_secs, sample_rate, channels, peak }` |

`media_import` decodes the file (proving it is audio), re-encodes it as Float32
WAV under a freshly generated name inside `media/`, and registers it as an
undoable source add. An import can never overwrite existing media.

## 3. Tracks & clips

| Tool | Tier | Inputs | Result |
|------|------|--------|--------|
| `track_list` | Observe | — | `[TrackSummary]` (`id`, `name`, `index`, `gain`, `pan`, `muted`, `solo`, `clip_count`, …) |
| `track_add` | Edit | `name` | `{ track_id }` |
| `track_update` | Edit | `track_id`, plus any of `name`, `gain`, `pan`, `muted`, `solo`, `armed` | `{ updated: true }` |
| `track_remove` | Edit | `track_id` | `{ removed: true }` |
| `clip_add` | Edit | `track_id`, `source_id`, `start_secs`, `duration_secs?`, `source_offset_secs?`, `gain?` | `{ clip_id, track_id, source_id, start_secs, duration_secs, source_offset_secs, gain }` |
| `clip_update` | Edit | `clip_id`, plus any of `start_secs`, `gain`, `fade_in_secs`, `fade_out_secs` | `{ updated: true }` |
| `clip_split` | Edit | `clip_id`, `split_secs` | `{ split: true, split_secs }` |
| `clip_remove` | Edit | `clip_id` | `{ removed: true }` |
| `range_delete` | Edit | `start_secs`, `end_secs` | `{ deleted: true }` — ripple-deletes on the first track |
| `marker_add` | Edit | `name`, `position_secs` | `{ marker_id }` |
| `edit_batch` | Edit | `label`, `operations[]` | `{ label, operations_applied, single_undo_step, status }` |

`clip_add` interprets `duration_secs`/`source_offset_secs` at the source's own
sample rate and `start_secs` at the project rate. Overlaps are legal — the mix
engine sums them, and the response carries the written range so a caller can
detect them.

### `edit_batch`

Applies an array of operations in order. Either every operation applies or none
do: a failure rolls the batch back and the error says `no changes were applied`.
A successful batch is exactly one undo entry.

```json
{
  "label": "intro cleanup",
  "operations": [
    { "op": "track_update", "track_id": "...", "name": "Narration", "gain": 0.8 },
    { "op": "clip_split", "clip_id": "...", "split_secs": 12.5 },
    { "op": "marker_add", "name": "Chapter 1", "position_secs": 12.5 }
  ]
}
```

Operation kinds: `track_update`, `clip_update`, `clip_remove`, `clip_split`,
`marker_add`, `range_delete`, `effect_apply`, `effect_remove`.

## 4. Effects

| Tool | Tier | Inputs | Result |
|------|------|--------|--------|
| `effect_catalog` | Observe | — | `{ effects: [EffectDescriptor], presets: [PresetDescriptor] }` |
| `effect_apply` | Edit | `track_id`, `effect_id`, `params?` | `{ effect_id }` (the chain entry handle) |
| `effect_update` | Edit | `track_id`, `effect_id`, `params` | `{ updated: true }` |
| `effect_remove` | Edit | `track_id`, `effect_id` | `{ removed: true }` |
| `preset_apply` | Edit | `track_id`, `preset` | `{ applied: true }` |

`effect_apply` and `preset_apply` validate the id against the catalog and fail
with the known list. Presets available: `voice_cleanup`, `podcast_master`.
Note that the offline renderer does not yet process track effect chains, so a
chain change is a project change, not an audible one.

## 5. Smart edits

All four measure the render, edit the timeline with ripple-delete commands, then
re-render and report the measured before/after.

| Tool | Tier | Inputs | Result |
|------|------|--------|--------|
| `edit_trim_silence` | Edit | `track_id`, `threshold_db`, `min_silence_secs` | `{ silence_trimmed_secs, ranges_cut, before_secs, after_secs, threshold_dbfs, min_silence_secs }` |
| `edit_shorten_pauses` | Edit | `track_id`, `max_pause_secs`, `crossfade_ms?` | `{ pauses_processed, seconds_removed, before_secs, after_secs, max_pause_secs }` |
| `edit_remove_fillers` | Edit | `track_id`, `filler_word_ids?` | `{ fillers_removed, fillers_found, before_secs, after_secs, engine }` |
| `edit_match_loudness` | Edit | `target_lufs` | `{ target_lufs, before_lufs, actual_lufs, peak_dbfs, gain_db, limited_by_peak }` |

- `threshold_db` must be in `-96..=0`; `min_silence_secs` may be 0.
- `edit_shorten_pauses` keeps `max_pause_secs` of each long pause and cuts the
  excess from the end of the pause, so the next onset is preserved.
- `edit_remove_fillers` needs word-level timestamps. With the built-in
  segmenter (placeholder words only) it **refuses** with an explanation instead
  of cutting at invented positions. Defaults: `um, uh, er, uhm, hmm, mmm`.
- `edit_match_loudness` scales every track's gain, re-measures, and reduces the
  gain if the sample peak would pass −1 dBFS; `limited_by_peak` says when that
  happened. It refuses on a project that renders to digital silence.

## 6. Analysis

All analysis runs on a real render of the project through the engine.

| Tool | Tier | Inputs | Result |
|------|------|--------|--------|
| `analyze_loudness` | Observe | — | `{ duration_secs, integrated_lufs, peak_dbfs, noise_floor_dbfs, active_ratio, clipping_spikes, detected_hum_hz }` |
| `analyze_silence` | Observe | — | `{ duration_secs, silence_count, total_silence_secs, avg_silence_secs, max_silence_secs, threshold_dbfs, min_silence_secs }` |
| `analyze_quality` | Observe | — | `{ integrated_lufs, peak_dbfs, noise_floor_dbfs, clipping_spikes, detected_hum_hz, active_ratio }` |
| `analyze_spectrum` | Observe | — | `{ bins, magnitudes, magnitudes_dbfs, blocks, sample_rate }` |
| `render_waveform` | Observe | `width`, `height` | `{ ascii, width, height, peak, duration_secs }` |
| `render_spectrogram` | Observe | `width`, `height` | `{ ascii, width, height, band_centers_hz, range_db }` |
| `compare_renders` | Observe | `path_a`, `path_b` | `{ similarity, differences, recommendation }` |

Field meaning:

- `peak_dbfs` is the **sample peak**. Inter-sample true peak is not measured.
- `noise_floor_dbfs` is the 10th-percentile 20 ms frame level.
- `active_ratio` is the fraction of 20 ms frames above −45 dBFS.
- `analyze_silence` uses −30 dBFS and ignores dips shorter than 100 ms.
- `analyze_spectrum` probes 16 log-spaced bands (Goertzel) averaged over up to
  64 blocks across the whole render — band energy, not an FFT bin dump.
- `detected_hum_hz` is `None`: narrowband mains detection is not implemented,
  and guessing would be worse than admitting it.

`render_waveform` returns the art in both the text summary and
`structuredContent.ascii`; rows are printed top-down, so the loudest cells sit
low in the picture. `render_spectrogram` prints low frequencies at the bottom
and maps −60 dB of range onto ` .:-=+*#%@`.

## 7. Voice, TTS & transcription

Engines run daemon-first and results always name what actually ran:
`daemon:<model>` when the `alp-infer` worker served the request,
`builtin-acoustic`/`builtin` for the in-process fallback.

| Tool | Tier | Inputs | Result |
|------|------|--------|--------|
| `tts_voices` | Observe | — | `{ voices, daemon_alive, daemon_active_model }` |
| `voice_profile_get` | Observe | `voice_id` | profile detail (no embedding; consent as a boolean) |
| `tts_generate` | Edit | `text`, `voice_id`, `speed?` | `{ path, duration_secs, engine, sample_rate, job_id }` |
| `tts_script_generate` | Edit | `script_json` | `{ track_id, clip_ids, engines, total_duration_secs, end_secs, single_undo_step, job_id }` |
| `tts_regenerate` | Edit | `clip_id`, `new_text`, `voice_id`, `speed?` | `{ clip_id, new_clip_id, new_text, engine, duration_secs, start_secs, single_undo_step }` |
| `voice_design` | Edit | `name`, `prompt`, `tags?` | a profile detail; `kind` is always `designed` |
| `transcribe` | Observe | `path`, `format?`, `out?` | `{ engine, duration_secs, text, segments, transcript_path, job_id }` |

- `tts_generate` writes a 16-bit WAV into the project `media/` directory and
  reports a `job_id` you can poll with `job_status`. Import it and `clip_add` to
  place it.
- `tts_script_generate` accepts `{"segments":[…]}` or a bare array of
  `{ text, voice_id, speed?, gap_secs? }`. It synthesizes each line, imports it,
  and lays clips out on the `Script` track with `gap_secs` (default 0.35)
  between them.
- `tts_regenerate` replaces the clip's audio at its existing timeline position
  and keeps the surrounding clips where they are.
- `voice_design` can only produce designed profiles. **Clone creation is
  human-only** (GUI, with a recorded consent) and no MCP tool can create,
  import or export a clone.
- `transcribe` writes `text`/`srt`/`vtt` when `out` is given. With the built-in
  segmenter the text is placeholder (`Speech segment N`) and `engine` says
  `builtin`.

## 8. Export

| Tool | Tier | Inputs | Result |
|------|------|--------|--------|
| `export_presets` | Observe | — | `{ presets: [{ id, label, description, sample_rate, bit_depth }] }` |
| `export_audio` | Export | `preset`, `output_path` | `{ exported_path, … }` |
| `export_stems` | Export | `output_dir`, `preset?` | `[StemExportReport]` |
| `export_transcript` | Export | `format`, `path` | `{ format, path, duration_secs, engine, segments, text }` |
| `preview_render` | Observe | `start_secs`, `duration_secs` | `{ start_secs, duration_secs, sample_rate, channels, peak, preview_path }` |

- Presets: `podcast`, `audiobook`, `voice_over`. An unknown preset is rejected,
  not silently mapped to a delivery default.
- `export_stems` renders each track **in isolation** and writes
  `stem_NN_<track>.wav`; the report carries that stem's own `peak_dbfs`, so you
  can verify a quiet track stayed quiet.
- `export_transcript` renders the project, transcribes it, and writes real
  SRT/VTT cues. The report echoes the file content in `text`.
- `preview_render` writes a WAV slice into `media/` and reports that slice's own
  peak.
- Exports that contain any synthetic voice source are written with the
  synthetic-audio watermark; no MCP tool can disable it.

## 9. Jobs

| Tool | Tier | Inputs | Result |
|------|------|--------|--------|
| `job_status` | Same as job | `job_id` | `{ id, label, state, progress, detail }` |
| `job_cancel` | Same as job | `job_id` | `{ cancelled: true }` |

Jobs are created by `tts_generate`, `tts_script_generate` and `transcribe`.
Their report carries `job_id`; the work has already completed by the time the
call returns, so polling shows `completed` with the produced artefact in
`detail`. Cancelling a completed or unknown job is an error.

## 10. Plugins

| Tool | Tier | Inputs | Result |
|------|------|--------|--------|
| `plugin_list` | Observe | — | discovered CLAP/LV2 plugins with parameters |
| `plugin_insert` | Edit | `track_id`, `plugin_id` | `{ effect_id }` |
| `plugin_param_set` | Edit | `track_id`, `effect_id`, `param_name`, `value` | `{ updated: true }` |

## 11. Resources & prompts

| URI / prompt | Contents |
|--------------|----------|
| `alp://project/active` | the active project summary as JSON |
| `alp://voices` | the `tts_voices` report as JSON |
| `alp://effects/catalog` | the live `effect_catalog` report as JSON |
| `alp://docs/agent-guide` | `docs/mcp/AGENT_GUIDE.md`, served from the file itself |
| `clean_up_voice_recording` | recipe: measure → checkpoint → preset → trim → match loudness → verify → export |
| `produce_podcast_episode` | recipe: create/open → import → place → mix → master → stems + delivery |
