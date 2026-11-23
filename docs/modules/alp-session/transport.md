# `alp-session` — `transport`

## Purpose

`transport` is the headless audio playback coordinator, meter streaming engine, and multi-resolution peak pyramid extractor. It wires the desktop application (both iced and Tauri 2) to [`alp_engine`] for real-time playback, tracks playhead positions with millisecond timestamp anchors for client-side interpolation, generates calibrated meter frames (-60 dB to +6 dB), and builds min/max peak pyramids (256, 1024, 4096 frames/bucket) for high-performance canvas waveform rendering.

## Public API

- `DEFAULT_SAMPLE_RATE`, `BASE_BUCKET`, `LEVEL_RATIO` constants.
- `PeakLevelDto { bucket_samples, pairs }`: one resolution level of a peak pyramid with `(i16, i16)` min/max pairs.
- `WaveformPeaksDto { sample_rate, frames, duration_secs, levels }`: complete multi-level peak pyramid with `level_for_zoom(samples_per_pixel)`.
- `MeterFrame { peak_l, peak_r, rms_l, rms_r, position_secs, anchor_timestamp_ms, playing, duration_secs }`: instantaneous streaming meter frame.
- `TransportState { playing, position_secs, position_frames, duration_secs, sample_rate, loop_enabled, loop_start_secs, loop_end_secs, volume, master_peak_l, master_peak_r, master_rms_l, master_rms_r, anchor_timestamp_ms }`: comprehensive transport snapshot.
- `TransportController`:
  - `new(duration_secs, sample_rate)`: constructor.
  - `state()`: snapshot getter.
  - `play(&mut session)`: start audio playback via `alp_engine::play_scene_live` or virtual transport clock fallback.
  - `pause()`: pause playback, preserving position.
  - `stop()`: stop playback and rewind to start or loop start boundary.
  - `seek(position_secs)`: seek to target timestamp.
  - `set_volume(volume)`: set master output volume fader (0.0..=1.5).
  - `set_loop(enabled, start, end)`: configure timeline loop playback.
  - `tick()`: poll hardware/virtual meters and construct a `MeterFrame`.
  - `build_peaks_for_session(session)`: extract 3-level peak pyramid from session audio.
  - `synthetic_1hour_peaks()`: generate 1-hour waveform benchmark data (172,800,000 frames @ 48 kHz).

## Data flow

```text
EditSession ──► MixScene ──► alp_engine::play_scene_live ──► PlaybackHandle
                                                                  │
UI Channel ◄── MeterFrame ◄── TransportController::tick ◄────────┘
     │
     └──► Client Interpolation: pos = anchor_pos + (now - anchor_ts) / 1000
```

1. Playback starts on an off-thread audio stream via `alp_engine`. If no physical device is detected (headless/container/CI), a virtual transport clock activates automatically.
2. `TransportController::tick` samples device/virtual meters at ~30 Hz, packing left/right peaks, RMS values, position, and anchor timestamp into a `MeterFrame`.
3. The frontend interpolates the playhead via `requestAnimationFrame` at 60–120 fps between incoming meter anchors.

## Allowed dependencies

- `alp-core`, `alp-engine`, `alp-media`, `serde`.
- `std` (`time::{Duration, Instant}`, `sync::Arc`).
- Never depends on `iced`, `tauri`, `alp-ui`, or `alp-ui-kit`.

## Threading / real-time notes

- Audio playback and device callbacks run strictly on background threads managed by `alp_engine`.
- Audio callback never allocates, locks, or performs I/O.
- IPC meter streaming runs on a decoupled timer (~33 ms / 30 Hz), preserving 0% CPU consumption when idle.

## Error handling

- Invalid seek times and volume values are clamped to valid ranges.
- Device initialization failures fall back gracefully to virtual transport clock without panics or crashes.
- Session lock poisoning errors are caught and surfaced via standard `Result<T, String>`.

## Testing

- `test_transport_initial_state`: validates default state values.
- `test_transport_play_pause_stop_seek`: exercises the full transport state machine.
- `test_transport_loop_wrapping`: verifies boundary wrapping under active playback.
- `test_synthetic_1hour_peaks_levels_and_buckets`: validates 675,000 bucket geometry and zoom level selection.
- `test_10_minute_playback_soak_no_dropped_callbacks`: simulates 10 minutes of audio monitoring with zero dropped frames.

## Known limits

- Only one active playback stream is tracked simultaneously per controller instance.
- Peak pyramids currently represent mono-summed peak envelopes.
