# `alp-core::project`

## Purpose

Defines the serialisable project graph: sources, tracks, clips, envelopes and
effect references, plus validation and frame-to-time helpers. This is the data
that every other crate consumes.

## Public API

- `Project`: fields `id`, `name`, `sample_rate`, `created_at_ms`, `updated_at_ms`,
  `sources`, `tracks`; methods `new`, `new_with_rate`, `new_with_id`, `touch`,
  `source`, `source_mut`, `track`, `track_mut`, `clip`, `clip_mut`,
  `track_of_clip`, `duration_frames`, `clip_timeline_len`, `clip_timeline_end`,
  `clip_count`, `validate`
- `Source`: `id`, `name`, `relative_path`, `sample_rate`, `channels`, `frames`
- `Track`: `id`, `name`, `gain`, `pan`, `muted`, `solo`, `color`, `clips`
- `Clip`: `id`, `source_id`, `source_range`, `timeline_start`, `gain`,
  `fade_in_frames`, `fade_out_frames`, `envelope`, `effects`; `len` (source
  frames), `is_empty`, `timeline_end` and `timeline_range` (exact when source
  and project rates match; use the `Project` methods for mixed rates)
- `EnvelopePoint { frame, gain }`
- `EffectRef { id, kind, enabled, params }` with `params: BTreeMap<String, f64>`
- `DEFAULT_SAMPLE_RATE` (48 kHz), `MIN_SAMPLE_RATE`, `MAX_SAMPLE_RATE`
- `now_ms()`

## Data flow

A new project generates a `ProjectId` and timestamps. Callers mutate through
commands. `validate` enforces unique ids, valid sample rate, non-empty names,
finite gains/pans, ordered envelopes, clips that fit their source, and no
duplicate clip/source references. `clip_timeline_len` and
`clip_timeline_end` scale a mixed-rate source’s native frames to project
frames; `duration_frames` is the maximum timeline end.

## Allowed dependencies

`serde`, `uuid` and `CoreError`. No other ALP crate.

## Threading / real-time notes

Plain owned data, not shared with the audio callback. Serialising or validating
a large project allocates; use a worker. The model is `Send` but not internally
synchronised.

## Error handling

Invalid sample rates and ranges get dedicated errors; structural problems get
`Validation`. `validate` is called on load and save, so a malformed file cannot
reach the engine.

## Testing

Tests cover the empty project and its zero duration, sample-rate rejection,
out-of-range clip data and duplicate IDs. Track/clip lookup, envelope ordering
and gain/pan finiteness rely on validation and other tests.

## Known limits

- One sample rate per project; mixed-rate sources are resampled at playback.
- `EffectRef` parameters are opaque JSON and not applied in Phase 1.
- Timestamps use wall-clock milliseconds, not a monotonic clock.
