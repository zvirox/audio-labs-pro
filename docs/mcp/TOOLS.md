# Audio Labs Pro — MCP Tool Reference

Model Context Protocol (MCP) tools exposed by `alp-mcp`. Tools are filtered dynamically based on the active session's permission tier.

---

## 1. Project & History Management

### `project_info` (Tier: Observe)
Returns metadata and status for the current project.
- **Inputs:** None
- **Outputs:** `{ id, name, revision, sample_rate, duration_secs, track_count, clip_count, undo_steps, redo_steps, is_dirty }`

### `project_save` (Tier: Edit)
Atomically saves the project and flushes recovery caches.
- **Inputs:** None
- **Outputs:** `{ saved: true }`

### `history_undo` (Tier: Edit)
Reverts the most recent timeline command.
- **Inputs:** None
- **Outputs:** `{ undone: bool }`

### `history_redo` (Tier: Edit)
Reapplies the most recently undone command.
- **Inputs:** None
- **Outputs:** `{ redone: bool }`

### `checkpoint_create` (Tier: Edit)
Captures an immutable snapshot of the project for rollback.
- **Inputs:**
  - `label` (string, required): description for the checkpoint.
- **Outputs:** `{ checkpoint_id: string }`

### `checkpoint_restore` (Tier: Edit)
Restores project state to a previously captured checkpoint.
- **Inputs:**
  - `checkpoint_id` (string, required): checkpoint handle.
- **Outputs:** `{ restored: true }`

### `rollback_session` (Tier: Edit)
Restores the project to the initial checkpoint captured at session start ("Undo all AI changes").
- **Inputs:** None
- **Outputs:** `{ rolled_back: bool }`

---

## 2. Tracks & Timeline Editing

### `track_list` (Tier: Observe)
Lists all tracks on the timeline with gain, pan, mute, and solo state.
- **Inputs:** None
- **Outputs:** Array of `TrackSummary` objects.

### `track_add` (Tier: Edit)
Adds a new track to the timeline.
- **Inputs:**
  - `name` (string, required): name of the new track.
- **Outputs:** `{ track_id: string }`

### `track_remove` (Tier: Edit)
Removes an existing track by ID.
- **Inputs:**
  - `track_id` (string, required): track handle.
- **Outputs:** `{ removed: true }`

### `clip_split` (Tier: Edit)
Splits a clip at a fractional second timestamp.
- **Inputs:**
  - `clip_id` (string, required): clip handle.
  - `split_secs` (number, required): timeline split position in seconds.
- **Outputs:** `{ split: true, split_secs: number }`

### `range_delete` (Tier: Edit)
Deletes an audio range and ripples subsequent audio left.
- **Inputs:**
  - `start_secs` (number, required): start position in seconds.
  - `end_secs` (number, required): end position in seconds.
- **Outputs:** `{ deleted: true }`

---

## 3. Acoustic Analysis

### `analyze_loudness` (Tier: Observe)
Measures ITU-R BS.1770 LUFS integrated loudness, true peak, and silence ratio.
- **Inputs:** None
- **Outputs:** `{ duration_secs, integrated_lufs, true_peak_dbfs, noise_floor_dbfs, silence_ratio, clipping_spikes }`

---

## 4. Media & Clips

Media files live in the project's `media/` directory; imports ride the undoable command stack like every edit.

### `media_list` (Tier: Observe)
Lists imported project media sources.
- **Inputs:** None
- **Outputs:** `{ media: [{ id, name, relative_path, sample_rate, channels, frames, duration_secs, peak, file_exists }] }`

### `media_import` (Tier: Edit)
Imports a sandboxed audio file into the project media dir and registers it as a source. Files are decoded (proving they are audio), re-encoded as Float32 WAV, and stored under a fresh generated name - an import can never overwrite existing media.
- **Inputs:**
  - `path` (string, required): audio file path inside the workspace sandbox.
  - `name` (string, optional): display name (defaults to the file's stem).
- **Outputs:** `{ source_id, name, relative_path, path, duration_secs, sample_rate, channels, peak }`

### `clip_add` (Tier: Edit)
Places media on a track as a new clip appended to the track's clip list. Source offsets and durations are interpreted at the source's own sample rate; the timeline start uses the project rate. Overlapping clips are legal (the mix engine sums them) - the response carries the written range so callers can detect overlaps.
- **Inputs:**
  - `track_id` (string, required): track handle (see `track_list`).
  - `source_id` (string, required): media source handle (see `media_list`).
  - `start_secs` (number, required): timeline start in seconds.
  - `duration_secs` (number, optional): clip length (default: whole remaining source).
  - `source_offset_secs` (number, optional): offset into the source (default 0).
  - `gain` (number, optional): linear clip gain, 0.0 to 4.0 (default 1.0; clamped).
- **Outputs:** `{ clip_id, track_id, source_id, start_secs, duration_secs, source_offset_secs, gain }`

---

## 5. Voice & Transcription

Engines run daemon-first: results always name the engine that actually ran (`daemon:<model>` when the `alp-infer` worker served the request, `builtin-acoustic` / `builtin` when the in-process fallback did). The built-in STT is an energy/formant segmenter that emits placeholder `Speech segment N` words; results admit this whenever it runs.

### `tts_voices` (Tier: Observe)
Lists known voice profiles and daemon synthesis state. Cloned profiles are visible but can never be created or imported through MCP.
- **Inputs:** None
- **Outputs:** `{ voices: [{ id, name, kind: "builtin"|"designed"|"cloned", backend, tags }], daemon_alive: bool, daemon_active_model: string|null }`

### `voice_profile_get` (Tier: Observe)
Inspects one voice profile. `cached_embedding` is never exposed; consent is reported only as a boolean.
- **Inputs:**
  - `voice_id` (string, required): voice profile handle (see `tts_voices`).
- **Outputs:** `{ id, name, kind, backend, tags, default_params: { speed, pitch, energy, emotion }, created_at, has_reference_audio, consent_recorded }`

### `tts_generate` (Tier: Edit)
Synthesizes speech and writes a 16-bit WAV into the project `media/` directory. Every profile known to the store is accepted; the built-in acoustic engine renders any profile with a generic timbre, so the `engine` field is the source of truth for what produced the audio. To place the result on the timeline, import it (`media_import`) and add a clip (`clip_add`).
- **Inputs:**
  - `text` (string, required): text to speak.
  - `voice_id` (string, required): voice profile handle.
  - `speed` (number, optional): speed multiplier, 0.25 to 4.0 (default 1.0).
- **Outputs:** `{ path, duration_secs, engine, sample_rate }`

### `voice_design` (Tier: Edit)
Designs a new voice from a natural-language brief and persists it to the profile store. Designed profiles only: cloned voice creation and import are never available through MCP at any tier.
- **Inputs:**
  - `name` (string, required): friendly name for the voice.
  - `prompt` (string, required): descriptive brief, e.g. `"warm, deep BBC documentary narrator with calm pacing"`.
  - `tags` (array of string, optional): category tags.
- **Outputs:** a `voice_profile_get` shape for the new profile (`kind` is always `"designed"`, `consent_recorded` always `false`).

### `transcribe` (Tier: Observe)
Transcribes a workspace audio file, daemon-first. Optionally writes the transcript to disk.
- **Inputs:**
  - `path` (string, required): audio file path inside the workspace sandbox.
  - `format` (string, optional): `"text"` | `"srt"` | `"vtt"` for the `out` file (default `"text"`).
  - `out` (string, optional): destination path for the transcript file (inside the sandbox).
- **Outputs:** `{ engine, duration_secs, text, segments: [{ start_secs, end_secs, text }], transcript_path: string|null }`

---

## 6. Master Audio Export

### `export_audio` (Tier: Export)
Renders timeline master audio to disk using a delivery preset.
- **Inputs:**
  - `preset` (string, required): `"podcast"` | `"audiobook"` | `"voice_over"`
  - `output_path` (string, required): destination file path within workspace.
- **Outputs:** `{ exported_path: string }`
