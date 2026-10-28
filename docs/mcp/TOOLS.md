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

## 4. Master Audio Export

### `export_audio` (Tier: Export)
Renders timeline master audio to disk using a delivery preset.
- **Inputs:**
  - `preset` (string, required): `"master"` | `"podcast"` | `"voice_over"`
  - `output_path` (string, required): destination file path within workspace.
- **Outputs:** `{ exported_path: string }`
