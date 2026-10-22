# `alp-ui-kit` — `waveform`

## Purpose

`waveform` turns a min/max peak pyramid into a scrollable, zoomable canvas
without depending on `alp-media`. It renders only the columns in the viewport,
which keeps a one-hour source inside the frame budget and makes the Phase 2
60 fps acceptance measurable.

## Public API

- `PeakLevel::new(bucket_samples, pairs)`; accessors `bucket_samples`, `pairs`,
  `len`, `is_empty`.
- `WaveformError` with `Display` and `Error`.
- `WaveformPeaks::new(sample_rate, frames, levels)` and accessors `sample_rate`,
  `frames`, `duration_secs`, `levels`, `level_for`, `peak_between`, `buckets`,
  `bytes`.
- `WaveformWidget` with `MIN_VISIBLE_SECS`, `MAX_VISIBLE_SECS`, `new`,
  `set_playhead`, `set_selection`, `visible`, `scroll_by`, `zoom_by` and
  `view<Message>()`.

## Data flow

`alp-ui::demo::convert` copies `alp_media::PeakPyramid` levels into owned
`PeakLevel` pairs and validates them. `WaveformWidget` stores the peaks behind an
`Arc`, the scroll offset and the visible duration. `view` creates a canvas
`Program`; `draw` selects a level via `level_for(samples_per_pixel)`, aggregates
each pixel column with `peak_between`, and paints a one-pixel bar whose colour
shifts from cool to hot with energy. Grid ticks come from `nice_tick`.

## Allowed dependencies

`iced` canvas/renderer types, `std::sync::Arc`, and the crate's `fonts` and
`theme` modules. It must not depend on `alp-media`; conversion is the caller's
job.

## Threading / real-time notes

Peaks are immutable after construction and shared read-only with the renderer.
Drawing runs on the UI/render thread. The loop is O(viewport width) and touches
only the buckets intersecting each column; it never walks all source samples.
No audio callback work.

## Error handling

`PeakLevel::new` rejects a zero bucket. `WaveformPeaks::new` rejects zero sample
rate/frames, no levels, and levels not strictly ordered fine-to-coarse. The demo
conversion uses `expect` because `alp-media` guarantees those invariants. Scroll
and zoom clamp to the source; a zero-frame source cannot be constructed.

## Testing

Five tests cover invalid pyramids, level selection at pixel scales, bucket
aggregation, scroll/zoom clamping and frame-accurate duration. The one-hour
release run records frame rate and frame time in the Phase 2 results.

## Known limits

- The widget is display-only: clicking does not seek, drag does not select, and
  the selection range is not yet wired to a pointer.
- `MAX_VISIBLE_SECS` is one hour; a longer source needs a larger cap or a
  different overview mode.
- Peaks are i16; float sources are normalized before peak building elsewhere.
- Canvas text uses a 9–10 px fixed size and does not scale independently of the
  window.
- It draws on the UI canvas through iced's renderer; there is no separate
  compute shader or off-thread GPU upload.
