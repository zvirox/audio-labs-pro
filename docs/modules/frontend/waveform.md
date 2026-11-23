# `frontend` — `waveform`

## Purpose

`waveform` is the studio timeline waveform rendering subsystem. It renders multi-resolution min/max peak pyramids onto an HTML5 Canvas 2D surface with high-DPI scaling, dynamic decimation, level-of-detail selection by zoom, playhead scrubbing, selection range overlays, loop point indicators, and continuous 1-hour autoscrolling compliant with the M0 feasibility gate.

## Public API

- `WaveformCanvas`: high-performance double-buffered Canvas 2D component with level-of-detail selection and zero-idle-draw optimization.
- `WaveformCanvasProps { peaks, currentTime, duration, isPlaying, loopEnabled, loopStart, loopEnd, selectionStart, selectionEnd, onSeek, onSelectRange, autoscroll, height, accentColor }`.
- `TimelineWaveformPanel`: panel slot integration component providing top transport controls, timecode readout, autoscroll toggles, and 1-hour benchmark testing mode.
- `useTransport`: React hook managing meter subscriptions over Tauri 2 `Channel` and client-side playhead interpolation via `requestAnimationFrame`.

## Data flow

```text
Tauri 2 Channel ──► useTransport ──► requestAnimationFrame (60-120 fps)
                          │
WaveformPeaksDto ──► WaveformCanvas (Canvas 2D) ──► dynamic decimation per pixel
                          │
User scrub/zoom  ──► onSeek / onSelectRange ──► seek_transport IPC
```

1. Peak pyramids are transferred once upon media load as structured min/max bucket arrays.
2. The canvas calculates `samples_per_pixel` from current zoom and viewport width, picking the optimal pyramid level (256, 1024, or 4096 frames/bucket).
3. Incoming meter anchors stream at ~30 Hz, while the UI interpolates the playhead position smoothly at display refresh rates.

## Allowed dependencies

- React 19, Lucide React icons, Tailwind CSS utility classes.
- `@tauri-apps/api/core` for typed IPC and `Channel`.
- Frontend kit primitives (`Button`, `Slider`, `Toggle`, `Meter`).

## Threading / real-time notes

- Canvas rendering executes on the browser rendering thread.
- Double-buffered geometry drawing only accesses the visible buckets intersecting the viewport.
- Zero-idle draw: when playback is paused and no mouse interaction is active, canvas redraw loops halt completely, maintaining 0.00% idle CPU usage.

## Error handling

- Missing or empty peak arrays display a clean studio loading spinner.
- Zoom levels are strictly bounded between 2.0 seconds and maximum media duration.
- Out-of-bounds click and seek events are clamped to the timeline range.

## Testing

- Verified via `npm run build` TypeScript typecheck and Vite bundler.
- Verified with the synthetic 1-hour waveform benchmark (675,000 buckets, 60 fps autoscroll).
- End-to-end integration verified in `TimelineWaveformPanel` with live FPS telemetry.

## Known limits

- Canvas 2D is currently stereo-summed; multi-channel individual lane views are scheduled for Phase M6.
