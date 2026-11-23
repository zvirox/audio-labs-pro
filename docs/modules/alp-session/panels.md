# `alp-session` — `panels`

## Purpose

`panels` is the pure headless floating-panel model. It owns the studio panel types, their
geometry, z-order, open/collapse state, docking side and the default responsive
layout. Keeping it independent of GUI widgets makes drag, snap and restore
testable without a display.

## Public API

- `PanelKind::ALL`, `id`, `title`; serialized as `snake_case`.
- `Point { x, y }`, `Size { width, height }`.
- `PanelRect { x, y, width, height }` with `right`, `bottom`, `clamp` and
  `snapped`; `MIN_WIDTH`, `MIN_HEIGHT`, `SNAP_PX`, `TITLE_BAR_HEIGHT`, `CHROME_HEIGHT`.
- `DockSide { Left, Right }`.
- `PanelState { kind, rect, z, collapsed, docked, open }`.
- `PanelManager` with `default_for`, `set_bounds`, `panel`, `panel_mut`,
  `active`, `set_open`, `toggle_open`, `focus`, `toggle_collapsed`, `drag_to`,
  `resize_to`, `settle`, `dock`, `undock`, `reset_geometry`, `painter_order`,
  `restore` and `snapshots`.

## Data flow

The application shell creates `PanelManager::default_for(window_size)` (or restores saved
snapshots), then maps pointer/keyboard input to manager calls. `drag_to`
uses the grab offset from cursor position and clamps into bounds;
`settle` snaps within `SNAP_PX` of an edge and raises z-order. `focus` raises a
panel and opens it. `painter_order` returns open panels lowest z first.

The default layout opens Transport, Tracks/Timeline, Inspector and Media Bin.
Voice Lab, Effects, Models, Export Queue, and AI Activity start closed.

## Allowed dependencies

- `serde` (`Serialize`, `Deserialize`).
- No GUI/windowing framework dependencies (zero `iced`, zero `tauri`).

## Threading / real-time notes

Pure synchronous state on the main coordination thread. No allocation in audio paths; the
panel vector is fixed. Coordinates are logical pixels.

## Error handling

`panel`/`panel_mut` use `expect` for a missing kind, which is an invariant of
`PanelKind::ALL`; restore silently ignores unknown or older kinds. All geometry
clamps, including negative sizes and out-of-bounds restores, and f32 `clamp`
ranges are constructed to be valid for windows at least as large as the minimum
size. There is no error result.

## Testing

Eight tests cover presence/open defaults, the narrow-window top row, drag snap,
z-order focus, collapse height, dock/undock, minimum resize and out-of-bounds
restore. Layout is deterministic at 1440x900 and 1163x821.

## Known limits

- Collapsing remembers only the minimum height, not the previous custom height.
- There is no rotation, grouping, tabbing or multi-monitor awareness.
- Docked panels occupy the full height below the chrome; they do not reserve
  space in a layout engine.
- Geometry is not rescaled proportionally when the window changes size; panels
  are clamped, which preserves pixels rather than relative position.
