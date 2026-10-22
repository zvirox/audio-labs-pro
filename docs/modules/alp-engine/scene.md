# `alp-engine::scene`

## Purpose

Converts a `Project` plus decoded sources into an immutable, playback-ready
`MixScene`. It also owns `SourceAudio`, resampling, channel adaptation and scene
validation. All allocation and resampling happen here, before the callback.

## Public API

- `SourceAudio::from_pcm`, `from_samples`, `sample_rate`, `channels`, `frames`,
  `samples`
- `SceneClip { source, source_start, source_end, timeline_start, gain,
  fade_in_frames, fade_out_frames, envelope }` with `len`, `is_empty`,
  `timeline_end`, `timeline_range`
- `MixScene::new`, `from_project`, `sample_rate`, `channels`, `clips`,
  `total_frames`, `duration_secs`, `with_channels`, `resampled`

## Data flow

`from_project` skips muted tracks; if any track is soloed, only soloed tracks
are included. It looks up each clip’s source, resamples it once per source when
the rate differs from the project rate, scales the clip’s source range, fades
and envelope to project frames, folds track gain into clip gain, validates the
result and builds the scene. Pan is ignored. `resampled` scales source ranges,
timeline positions, fades and envelope points for a device rate, clamping to the
resampled source length. `with_channels` only changes the output channel count;
the mixer maps mono/stereo sources.

## Allowed dependencies

`alp-core` (`Project`, ids, ranges, envelope), `alp-media` (`PcmBuffer`,
`resample`), `EngineError`.

## Threading / real-time notes

Construction decodes nothing itself, but may resample and allocate; call it on a
worker before playback. A finished `MixScene` is immutable and shared with the
callback. `Arc<SourceAudio>` keeps duplicate clips from copying samples.

## Error handling

`MixScene::new` rejects empty clips, out-of-source ranges, negative/non-finite
gains and fades longer than the clip. `from_project` errors when a source was
not preloaded or a clip points past the decoded source. Resampling failures
propagate as `EngineError::Media`.

## Testing

Tests cover muted-track skipping, oversize-range rejection, resampling duration,
fractional-rate clamping and mixed-rate project scaling. The CLI renders a
project with overlapping clips as an end-to-end check.

## Known limits

- Pan is not applied (Phase 5 effect chain).
- Effects are not instantiated.
- Clips are not sorted; the mixer scans all of them.
- Every source is decoded fully in memory.
- `with_channels` does not up/down-mix sources itself; the mixer handles mono
  and matching channels, leaving extra outputs silent.
