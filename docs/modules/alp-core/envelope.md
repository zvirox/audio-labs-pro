# `alp-core::envelope`

Volume and pan automation curve interpolation and evaluation.

## Purpose

Provides multi-point automation curves overlaid on audio clips in the multitrack timeline.
Each point specifies a relative frame offset and gain/pan multiplier. The envelope
evaluates values at arbitrary time positions using linear, smooth cosine S-curve, or
exponential interpolation.

## Public API

```rust
pub enum InterpolationMode {
    Linear,
    Smooth,
    Exponential,
}

pub struct EnvelopeCurve {
    pub points: Vec<EnvelopePoint>,
    pub mode: InterpolationMode,
}

impl EnvelopeCurve {
    pub fn new() -> Self;
    pub fn with_mode(mode: InterpolationMode) -> Self;
    pub fn with_points(points: Vec<EnvelopePoint>, mode: InterpolationMode) -> Self;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn clear(&mut self);
    pub fn add_point(&mut self, frame: u64, gain: f32);
    pub fn remove_point_at(&mut self, frame: u64, tolerance_frames: u64) -> bool;
    pub fn evaluate_at(&self, frame: u64, clip_len: u64) -> f32;
}
```

## Data flow

UI control point drags and edits update `EnvelopePoint` lists on `Clip`. During audio
rendering or interactive playback mixing, `Clip::evaluate_gain_at` evaluates the envelope
at the current frame offset and scales audio samples accordingly.

## Allowed dependencies

- `serde` for serialization.
- `crate::project::EnvelopePoint`.
- Core Rust standard library (`std`).

## Threading / real-time notes

Envelope evaluation is purely mathematical, allocation-free, and lock-free once the curve
is constructed. It operates cleanly inside real-time audio playback loops.

## Error handling

Frame lookups clamp gracefully to `[0.0, 1.0]`. Out-of-bound frames before the first point
or after the last point return the edge point's value. Empty curves return unity gain `1.0`.

## Testing

Unit tests in `crates/alp-core/src/envelope.rs` verify:
- Empty curves evaluate to `1.0`.
- Single control points produce constant gain.
- Linear and smooth interpolation accuracy across sample boundaries.
- Sorted insertion, update, and tolerance-based point removal.

## Known limits

Curves currently evaluate gain and pan automation; future iterations will support
spline tension handles and frequency filter automation.
