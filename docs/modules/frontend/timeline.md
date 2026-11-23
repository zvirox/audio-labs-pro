# `frontend/src/timeline`

## Purpose

High-performance HTML5 Canvas 2D multitrack timeline editor for non-linear audio arrangement, clip trimming, split razor cuts, fade curves, volume automation envelopes, magnetic snapping, and playhead scrubbing.

## Public API

- `MultitrackCanvas` — Core 2D canvas component rendering track lanes, clips, waveforms, fades, markers, loop ranges, and playhead beam.
- `MultitrackWorkspace` — Studio workspace integrating the canvas editor, editing toolbar (Split, Delete, Ripple, Snap, Zoom), timecode status, and keyboard shortcut routing.
- `hitTest(x, y, timeline, viewport)` — Framework-free mathematical hit tester detecting clips, trim boundaries, fade handles, markers, loop brackets, and ruler scrubbing.
- `createDragSession`, `updateDragSession`, `resolveDragCommand` — Framework-free drag state machine with magnetic snapping to markers, playhead, and adjacent clip edges.
- `timeToPixels`, `pixelsToTime`, `formatTimecode` — Bi-directional temporal-to-pixel coordinate projection utilities.

## Data flow

1. `MultitrackWorkspace` fetches authoritative timeline state from `getTimelineState` through the Tauri IPC bridge.
2. Pointer interactions on `MultitrackCanvas` evaluate hit targets and maintain an active `DragState`.
3. Pointer movement calculates magnetic snap targets against markers, loop boundaries, and clip edges.
4. On pointer release, `resolveDragCommand` maps the interaction to a typed `TimelineCommandPayload` (`move_clip`, `trim_clip`, `set_clip_fades`, `set_loop_range`, `set_selection`, `seek`).
5. IPC commands dispatch to Rust `alp-desktop` and `alp-session`, which execute undoable `alp_core::Command` objects.
6. Successful mutations trigger `session_revision_changed`, refreshing all canvas views across the app.

## Allowed dependencies

- `frontend/src/kit/*` design system primitives
- `frontend/src/ipc/types` and `frontend/src/ipc/client`
- Lucide React icons
- Standard DOM Canvas 2D Context API

## Threading / real-time notes

- Canvas rendering executes on the UI animation frame loop at up to 120 FPS.
- Pointer capture (`setPointerCapture`) guarantees uninterrupted drag sessions across window boundaries.
- Pure mathematical hit-testing and snapping modules run synchronously with sub-millisecond overhead.

## Error handling

- Coordinate projections clamp bounds to prevent negative or infinite durations.
- Over-trimming is constrained to a minimum 50ms clip duration threshold.
- Failed IPC commands revert ghost drag offsets and surface actionable notifications in the status bar.

## Testing

- Unit tests in `frontend/src/timeline/timelineMath.test.ts` (runnable via `npm test`) verify:
  - Exact round-trip coordinate projection identity (`timeToPixels` $\leftrightarrow$ `pixelsToTime`).
  - Hit testing precision across ruler, markers, loops, left/right trim zones, fade handles, and clip bodies.
  - Magnetic snapping within threshold and non-snapping outside threshold.
  - Safe trim and fade clamping without negative durations.
  - Resolution of drag sessions into valid IPC command payloads.

## Known limits

- Advanced spectral time-frequency brush selection is planned for Phase 13.
