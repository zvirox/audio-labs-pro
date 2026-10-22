# `alp-engine::device`

## Purpose

Enumerates cpal devices and plays a prepared `MixScene` on the default output
device. It is the only engine module that talks to cpal directly.

## Public API

- `DeviceDirection::{Output, Input}`
- `DeviceInfo { name, is_default, direction, channels, sample_rate,
  sample_format }`
- `list_devices()`
- `play_scene(scene, max_seconds)`
- `PlaybackReport { callbacks, frames, elapsed_secs, finished, stream_error }`

## Data flow

`list_devices` walks output and input devices, reads `description()` and the
default config, and returns plain data. `play_scene` gets the default output
device, resamples the scene to the device rate, reinterprets it at the device
channel count, builds a `Mixer`, and constructs an f32 or i16 stream. The
callback mixes directly (f32) or through a preallocated scratch buffer (i16) and
updates atomics. The caller sleeps until the scene finishes or `max_seconds`
elapses, then drops the stream.

## Allowed dependencies

`cpal`, `std::sync::atomic`, `std::time`, `Mixer`, `MixScene`, `EngineError`.

## Threading / real-time notes

- Callback: `Mixer::render_into`, conversion and atomics only.
- Scratch is `16_384 * channels` f32, so an unexpectedly large callback is
  processed in chunks without allocating.
- A backend error callback sets an atomic; it never logs from the audio thread.
- Device probing and scene preparation happen before `play`.

## Error handling

Missing default device → `NoDevice`; config/build/start failure → `Stream`;
unsupported sample format → `UnsupportedFormat`. A runtime backend error sets
`stream_error` in the report and does not panic.

## Testing

The i16 clamp helper is unit-tested. Playback is intentionally not part of CI;
`scripts/phase1-demo.sh --play` checks it on real hardware.

## Known limits

- Default device only.
- f32 and i16 sample formats only.
- No device hot-plug handling or stream restart.
- Pan is not applied by the mixer.
