# `alp-dsp`

## Purpose

Defines the effect boundary shared by live playback and offline rendering. Phase
1 ships the `Effect` trait and a reference linear `Gain` stage so the engine,
UI and export paths can be wired before the real processors exist.

## Public API

- `Effect` trait: `name`, `prepare`, `process`, `reset`
- `Gain::new`, `Gain::gain`, and its `Effect` implementation

## Data flow

The engine will call `prepare(sample_rate, channels)` before a stream, then
`process(&mut [f32])` on interleaved blocks in place. `reset` clears delay lines
and envelopes when the transport seeks. Phase 1 uses `Gain` only as a boundary
test; project clip gain is applied directly by the mixer.

## Allowed dependencies

`alp-core` only (for `CoreError` validation). No external DSP crate yet.

## Threading / real-time notes

`process` is intended to run on the audio callback, so implementations must not
allocate, lock, log or block. All state must be allocated in `prepare` or the
constructor. Phase 1 `Gain` is a single multiplication and is real-time safe.

## Error handling

`Gain::new` rejects non-finite and negative values with
`CoreError::validation`. Future processors will return a construction error and
keep per-block processing infallible.

## Testing

A unit test verifies gain scaling. Future effects will add impulse-response,
null-test and parameter-smoothing tests.

## Known limits

- No EQ, dynamics, denoise or loudness processors yet.
- No parameter automation, oversampling or latency reporting.
- `EffectRef` in `alp-core` is stored but not rendered.
