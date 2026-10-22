# `alp-engine`

## Purpose

Runs audio. It owns the real-time mixer, transport state, lock-free control
channel, cpal playback/capture and the offline mixdown used by export and the
CLI. It is the only crate allowed to touch cpal, and every callback path is
designed to run without allocation, locks, logging or syscalls.

## Public API

- `MixScene`, `SceneClip`, `SourceAudio`
- `Mixer` with `new`, `render_into`, `seek`, `position`, `total_frames`,
  `finished`
- `Transport`, `TransportState`
- `EngineCommand`, `CommandSender`, `CommandReceiver`, `command_channel`
- `list_devices`, `DeviceInfo`, `DeviceDirection`
- `play_scene`, `PlaybackReport`
- `record_to_wav`, `CaptureReport`
- `render_scene`, `render_scene_to_wav`, `RenderReport`, `DEFAULT_BLOCK_FRAMES`
- `EngineError`

## Data flow

1. The caller decodes sources with `alp-media` and wraps them in `SourceAudio`.
2. `MixScene::from_project` filters muted/soloed tracks, folds track gain into
   clip gain, resamples sources that differ from the project rate, and builds an
   immutable clip list.
3. For a device, `play_scene` resamples the scene to the device rate, widens or
   narrows it to the device channel count, and constructs `Mixer` before the
   stream starts.
4. The callback calls `Mixer::render_into` and updates atomics only.
5. `render_scene` allocates the complete output and runs the same mixer in
   blocks, guaranteeing that block size cannot change the result.

## Allowed dependencies

`alp-core` and `alp-media`. External crates are `cpal`, `rtrb`, `hound`,
`thiserror` and `tracing`. It must never depend on UI, voice or inference.

## Threading / real-time notes

- `Mixer::render_into` allocates nothing, takes no locks, logs nothing and does
  not touch the filesystem.
- Envelope cursors and the device scratch buffer are allocated before the
  stream starts.
- The control channel is an `rtrb` SPSC ring; producers never block.
- Capture copies samples into an SPSC ring; a writer thread does WAV I/O.
- The backend error callback only sets an atomic flag.

## Error handling

`EngineError` covers invalid scenes, missing or unsupported devices, stream
build/start failures, I/O and media failures. A backend stream error is reported
through `PlaybackReport::stream_error` / `CaptureReport::stream_error` and makes
the CLI exit non-zero. Device probing is fallible but never panics on a missing
device.

## Testing

Unit tests cover sample-accurate mixing, overlapping clips, fades, envelope
interpolation, block-size invariance, transport transitions and loop wrapping,
scene validation and resampling length. Device tests are intentionally omitted
from CI because hosted runners have no sound card; acceptance playback is run on
a real machine with `scripts/phase1-demo.sh --play`.

## Known limits

- Playback and capture use the default device only.
- Pan is stored but not applied; the effect chain owns it in Phase 5.
- Playback decodes all sources before the stream starts.
- Only f32 and i16 device formats are supported.
- Capture writes float32 WAV and does not resample.
