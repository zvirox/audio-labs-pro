# `frontend/src/panels`

## Purpose

Floating and docked panel container system for Audio Labs Pro. Enables multi-window studio workflows with dragging, resizing, collapse/minimize, z-order focus, and persistence to `workspace.json`.

## Public API

- `PanelContainer` — Draggable, resizable floating window container rendering any `PanelKind`.
- `PanelState`, `PanelRect`, `PanelKind`, `DockSide` — Data types synchronized with `alp_session::panels`.

## Data flow

1. Dragging a panel's title bar tracks mouse deltas and updates `panel.rect.x` and `panel.rect.y`.
2. Resizing updates `width` and `height`, constrained to minimum allowable bounds (260x140px).
3. Clicking anywhere on a panel raises its `z` index above all other panels.
4. Any geometry mutation triggers `saveWorkspaceState()` to persist the layout through Tauri IPC to disk.

## Allowed dependencies

- `frontend/src/ipc/types`
- `frontend/src/ipc/client`
- Lucide React icons

## Threading / real-time notes

Mouse move listeners are bound during active drag/resize operations and removed on mouse up to eliminate background event loop overhead.

## Error handling

Panel coordinates are clamped within the active window frame, preventing windows from getting lost offscreen. Corrupt or unparseable configurations safely fall back to default layouts.

## Testing

Verified with mouse drag and resize operations, collapse toggles, and layout reset roundtrips.

## Known limits

Multi-monitor detached window popping is out of scope; all floating panels reside within the single transparent OS window.
