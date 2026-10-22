# `alp-core` / `selection`

## Purpose

Transient timeline selection state: one track, an anchor, a moving cursor and a
frame grid.

## Public API

- `Selection::at(track_id, frame)`
- `Selection::between(track_id, anchor, cursor, snap_frames)`
- `track_id`, `anchor`, `cursor`, `snap_frames`, `range`, `len`, `is_empty`
- `set_snap_frames`, `snap`, `select_to`, `collapse`, `select_all`, `clamp_to`

## Data flow

The anchor stays fixed while the cursor moves. `range` orders both ends into a
half-open `FrameRange`; snapping rounds both ends to the nearest grid line.
Selections are not serialised with the project.

## Allowed dependencies

`alp-core` ids and time types only.

## Threading / real-time notes

Plain `Copy` data owned by the UI. It is never read by the audio callback.

## Error handling

Snap grids are clamped to at least one frame. `clamp_to` prevents a selection
from extending past the timeline.

## Testing

Tests cover ordered/reversed ranges, midpoint snapping, select-all and clamping.

## Known limits

- Single-track only; Phase 4 can add a multi-track selection set.
