# `alp-engine::transport`

## Purpose

A plain state machine for playback position, play/pause/stop, seeking and loop
ranges. It is shared by the UI, the device callback and the offline renderer
without locks.

## Public API

- `TransportState::{Stopped, Playing, Paused}`
- `Transport::new(total_frames)`, `state`, `position`, `total_frames`,
  `is_playing`, `play`, `pause`, `stop`, `seek`, `set_loop`, `loop_range`,
  `advance(frames)`

## Data flow

`play` starts from the current position, rewinding to zero if parked at the end.
`pause` only changes state while playing. `stop` returns to zero. `seek` clamps
to the project length. `set_loop` drops empty ranges. `advance` ignores input
unless playing; with a loop it wraps modulo the range length, otherwise it clamps
to the end and transitions to `Stopped`.

## Allowed dependencies

`alp_core::FrameRange`. No cpal, media or UI.

## Threading / real-time notes

Pure arithmetic with no allocation or locks; safe in a callback. In the current
Phase 1 device and offline paths the `Mixer` advances its own position;
`Transport` is ready for the UI/command wiring in Phase 2.

## Error handling

All methods are infallible. Invalid loop ranges are ignored (`None`). Seeking
past the end is clamped.

## Testing

Tests cover play/pause/stop, end-of-project auto-stop, seek clamping and loop
wrapping.

## Known limits

- No time signature, tempo or sample-accurate scheduling.
- No reverse playback or varispeed.
- `advance` is frame-count based, not wall-clock compensated.
