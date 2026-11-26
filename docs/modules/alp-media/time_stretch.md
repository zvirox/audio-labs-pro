# `alp-media` / `time_stretch`

## Purpose

WSOLA (Waveform Similarity Overlap-Add) duration change without pitch change. Used by dubbing rate fitting to make a rendered take match a subtitle/dialog window.

## Public API

- `time_stretch(&PcmBuffer, ratio: f32) -> Result<PcmBuffer, MediaError>`: `1.0` keeps duration, `1.5` makes it 50% longer, `0.75` makes it 25% shorter. Ratios must be in `(0, 4]`.

## Data flow

1. Frame the input with a 2048-sample Hann window and 1024-sample analysis hop.
2. Derive the synthesis hop from the stretch ratio.
3. For each output frame, pick the input offset whose leading samples best match the previous synthesised tail (search ±512 frames).
4. Overlap-add windowed frames and normalise by the accumulated window.

## Allowed dependencies

`alp-core` (buffer limits via `PcmBuffer`) and `crate::error`. No external crates or new dependency edges.

## Threading / real-time notes

Pure offline function. It allocates output and scratch vectors; never call it from the audio callback. Suitable for a job worker.

## Error handling

Non-finite, zero, or >4 ratios return `MediaError::Encode`. Buffers shorter than one analysis frame and unity ratios return a copy unchanged.

## Testing

Tests cover unity passthrough, longer/shorter durations, pitch preservation on a 220 Hz sine via zero-crossing rate, and invalid ratios.

## Known limits

- Offset matching uses the first channel only; the chosen shift is applied to all channels, which is standard for stereo WSOLA.
- Very large stretches (>2x) may introduce sampling artefacts; keep TTS speed adjustment as the first fitting tool.
- Processing is offline and can be slow for long takes.
