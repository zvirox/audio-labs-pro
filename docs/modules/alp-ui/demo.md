# `alp-ui` — `demo`

## Purpose

`demo` supplies deterministic synthetic peaks so the shell can be evaluated
without a decoder, audio device, model or project file. It uses the same
streaming `alp_media::PeakBuilder` as real imports and converts the result into
the toolkit's display type.

## Public API

- `DEFAULT_SECONDS` (600) and `MAX_SECONDS` (3,600).
- `configured_seconds()` — reads `ALP_DEMO_SECONDS`, clamped to 1..=3,600.
- `synth_peaks(seconds) -> Arc<WaveformPeaks>` — 48 kHz mono peaks.
- `convert(&PeakPyramid) -> WaveformPeaks` — the one boundary between
  `alp-media` and `alp-ui-kit`.
- `waveform(seconds, colors) -> (WaveformWidget, u64)` — builds both the view and
  the exact frame count used by the transport.

## Data flow

A sine lookup table is sampled in four chunks per second, with phrase, tremolo
and carrier envelopes, and pushed through `PeakBuilder`. `PeakBuilder::finish`
produces the three-level pyramid. `convert` copies the min/max pairs into
`WaveformPeaks`, and `WaveformWidget` owns it behind an `Arc` for drawing.

The environment override exists for the one-hour acceptance run:

```sh
ALP_DEMO_SECONDS=3600 ALP_DEMO_AUTOSCROLL=1 ALP_METRICS=1 \
  ALP_AUTOCLOSE_SECS=10 target/release/alp-ui
```

## Allowed dependencies

`alp-media` (peak builder and pyramid), `alp-ui-kit` (display peaks and widget),
`std::sync::Arc`. No decoder, audio device, inference runtime or file I/O.

## Threading / real-time notes

Peaks are generated synchronously during `App::boot`, before the first frame.
This is deliberate for a demo but is not the production import path: real
decoding and peak building must run on a worker. The resulting `WaveformPeaks`
is immutable and shared read-only with the render thread.

## Error handling

`synth_peaks` uses `expect` on builder and conversion results because the
synthetic chunks are frame-aligned by construction. `configured_seconds` treats a
missing or unparsable value as the default and clamps out-of-range values. A
zero-second request becomes one second.

## Testing

Tests verify the pyramid has three ordered levels, exactly 96,000 frames for two
seconds at 48 kHz, a positive byte size and a duration within one frame. The
1-hour release run is an acceptance check, not a unit test.

## Known limits

- Generation is O(samples) on the UI thread and takes noticeable startup time
  for a one-hour request; the default is ten minutes.
- The signal is a synthetic tone, not a realistic speech/music envelope.
- Memory is a small peak pyramid, not decoded audio, so it cannot exercise the
  mixer.
- `MAX_SECONDS` matches the acceptance window; longer sources need the real
  peak cache and streaming widget path.
