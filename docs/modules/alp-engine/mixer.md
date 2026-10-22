# `alp-engine::mixer`

## Purpose

The real-time sample mixer. It sums prepared clips at a timeline position,
applies clip/track gain, linear fades and volume automation, and is safe to call
from the audio callback.

## Public API

- `Mixer::new(scene)` — allocates envelope cursors
- `position`, `total_frames`, `finished`
- `seek(frame)`
- `render_into(&mut [f32]) -> usize`

## Data flow

`render_into` zeroes the output, computes the overlap of each clip with the
requested frame range, and for every frame adds the source sample scaled by
`clip.gain * fade * envelope`. Mono sources are copied to every output channel;
multi-channel sources fill the matching channels. Samples after the scene end
stay zero. The position advances by the requested frame count, capped at the
scene end.

## Allowed dependencies

`alp_core::EnvelopePoint`, `MixScene`, `SourceAudio`. No cpal, media or UI.

## Threading / real-time notes

This is the hot path. It allocates nothing, takes no locks, logs nothing and
does not touch the filesystem. Envelope cursors are preallocated. `assert!` on a
non-frame-aligned buffer is a programmer-error guard; the device always passes
aligned buffers.

## Error handling

There is no fallible result. Invalid scenes are rejected by `MixScene::new`
before a mixer exists. A source index beyond the buffer is skipped defensively.

## Testing

Tests cover overlapping clips, fades at both ends, envelope interpolation,
block-size invariance (one large call versus many small calls) and mono sources
mapped to stereo output.

## Known limits

- Linear fades and envelope interpolation only.
- No pan, effects, dither or sample-rate conversion inside the mixer.
- Every clip is scanned for every block; a future scheduler will index active
  clips only.
