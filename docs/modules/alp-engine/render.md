# `alp-engine::render`

## Purpose

Offline mixdown. It runs the same `Mixer` as playback into a fully allocated
buffer, so export and preview share one signal path and a null test is possible.

## Public API

- `DEFAULT_BLOCK_FRAMES` (1,024)
- `RenderReport { frames, duration_secs, peak, rms }`
- `render_scene(scene, block_frames) -> PcmBuffer`
- `render_scene_to_wav(scene, path, bit_depth) -> RenderReport`

## Data flow

`render_scene` allocates a silent `PcmBuffer` for the scene length, builds a
`Mixer`, and calls `render_into` on successive frame-aligned chunks. The block
size therefore cannot change the result. `render_scene_to_wav` renders, computes
peak/RMS, then writes atomically through `alp-media::write_wav`.

## Allowed dependencies

`alp-media` (`PcmBuffer`, `WavBitDepth`, `write_wav`), `Mixer`, `MixScene`,
`EngineError`.

## Threading / real-time notes

Not real-time: allocation, full-buffer memory and disk I/O. Run on a worker or
the CLI main thread. The mixer still avoids allocation per block.

## Error handling

`PcmBuffer::silence`, `render_scene` and `write_wav` failures propagate as
`EngineError::Media`. A zero block size is clamped to one. A zero-length scene
produces an empty buffer.

## Testing

Tests assert that block sizes 64 and 1,024 produce byte-identical buffers, that
frame count/peak/RMS are correct, and that a WAV render creates the file.

## Known limits

- Whole-buffer render; long projects use a lot of RAM.
- WAV output only.
- No progress callback or cancellation.
- No effects, pan or dither.
