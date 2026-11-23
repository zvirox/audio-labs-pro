# `frontend/src/layout`

## Purpose

The dual-axis application shell structure for Audio Labs Pro. Provides persistent primary workspace switching, contextual sub-navigation, docked transport playback, and collapsible property inspection.

## Public API

- `PrimaryRail` — 56px vertical rail on the left housing the 7 primary workspaces, theme toggle, and utility shortcuts.
- `SubtabTopBar` — 40px horizontal top bar displaying the active workspace title, contextual subtabs, and panel action menus.
- `InspectorDrawer` — 280px collapsible drawer on the right toggled via `Ctrl+B` for fine-grained track and clip editing.
- `DockedPlayer` — 64px bottom transport player bar featuring play/pause, stop, record, loop, digital timecode, timeline scrubber, and stereo meters.

## Data flow

1. Clicking an icon on the `PrimaryRail` updates the active workspace and triggers contextual subtab changes.
2. Spacebar or player controls dispatch playback state changes and update the digital timecode display.
3. Collapsing or expanding the inspector drawer adjusts the main canvas viewport width without altering floating panel coordinates.

## Allowed dependencies

- `frontend/src/kit`
- `frontend/src/ipc/types`
- Lucide React icons

## Threading / real-time notes

The transport player simulates real-time playback ticks at 100ms intervals when playing; in Phase M4 it binds directly to the `alp-engine` playhead anchor.

## Error handling

Drawer and rail boundaries are constrained to standard viewport dimensions, preventing clipping on smaller displays ($< 960$px).

## Testing

Tested across standard desktop display widths (1024px, 1280px, 1920px) under both Studio Dark and Light glass themes.

## Known limits

Docked player controls currently operate on demo timeline bounds (20.0s); full timeline length binding occurs with multitrack integration in Phase M6.
