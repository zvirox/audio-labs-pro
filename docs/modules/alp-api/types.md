# `alp-api` / `types`

## Purpose

Shared data types for the automation layer: permission tiers, approval modes,
project/track/clip summaries, the loudness report, and a timestamp formatter.
These are the shapes serialised to MCP clients, so field names and meanings are
part of the public contract.

## Public API

- `PermissionTier` — `Observe < Edit < Export` (ordered).
- `ApprovalMode` — `Auto`, `ConfirmDestructive`, `Review`.
- `ProjectSummary` — `id`, `name`, `revision`, `sample_rate`, `duration_secs`,
  `track_count`, `clip_count`, `undo_steps`, `redo_steps`, `is_dirty`.
- `TrackSummary` — `id`, `name`, `index`, `gain`, `pan`, `muted`, `solo`,
  `armed`, `clip_count`.
- `ClipSummary` — `id`, `track_id`, `source_id`, `start_secs`, `duration_secs`,
  `gain`, `fade_in_secs`, `fade_out_secs`.
- `AnalysisReport` — `duration_secs`, `integrated_lufs`, `peak_dbfs`
  (sample peak), `noise_floor_dbfs` (10th-percentile 20 ms frame level),
  `active_ratio` (fraction of frames above −45 dBFS), `clipping_spikes`,
  `detected_hum_hz`.
- `format_timestamp(secs: f64) -> String` — `hh:mm:ss.mmm`, clamped at zero.

## Data flow

1. `ApiSession` builds summaries from the `alp-core` model when a read tool is
   called.
2. Types are serialised straight into MCP `structuredContent`.
3. `PermissionTier` is compared with `>=` for gating; `ApprovalMode` is carried
   as policy data (no enforcement path yet).

## Allowed dependencies

- `serde::{Deserialize, Serialize}`

## Threading / real-time notes

Plain data and a pure formatter; no state, no synchronisation, no audio-thread
involvement.

## Error handling

Value types with no fallible constructors. Invalid values are rejected by the
operations that accept them, not by these structs.

## Testing

Unit tests cover the permission tier ordering and `format_timestamp` rounding
and clamping.

## Known limits

- `AnalysisReport.detected_hum_hz` is always `None` — no mains-hum detector.
- `peak_dbfs` is a sample peak; true peak is not measured.
- `undo_steps`/`redo_steps` are real undo/redo step counts and agree with
  `history_list`'s `total`.
- `ApprovalMode` is defined but no code path enforces it.
