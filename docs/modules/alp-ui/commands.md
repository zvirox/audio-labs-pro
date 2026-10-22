# `alp-ui` — `commands`

## Purpose

`commands` is the single command registry for the shell. The same data drives
the command palette, keyboard shortcuts and menu labels, so a command cannot be
discoverable in one surface but missing from another.

## Public API

- `CommandId` — every executable action, including `OpenPanel`/`ClosePanel`.
- `CommandEntry { id, title, category, hint }` and `registry()`.
- `filter(entries, query)` — case-insensitive title/category matching.
- `Shortcut` with `matches(modifiers, key)` and `label()`; `shortcut(id)`.
- `PaletteState` with `show`, `hide`, `set_query`, `move_selection` and
  `selected_command`.

## Data flow

`App::boot` copies `registry()` into its state. `Ctrl+K` opens the palette and
focuses its text input; typed text calls `PaletteState::set_query`, which filters
into indices. Arrow keys wrap the selection, Enter resolves the selected entry,
and the row buttons carry the same `RunCommand` message. Shortcut chords are
matched exactly, including absent modifiers, so `Space` does not fire while a
Ctrl chord is held.

## Allowed dependencies

`iced` keyboard types and `crate::panels::PanelKind`. It has no UI rendering
dependency; `overlays` renders the rows.

## Threading / real-time notes

Registry construction and filtering are O(number of commands) and run on the UI
thread only during input or palette opening. No allocation happens in a frame
loop beyond the view's normal strings.

## Error handling

An empty filtered list leaves `selected` at zero and `selected_command` returns
`None`; Enter becomes a no-op. Unknown shortcut ids return `None` rather than
panicking. An unknown key never matches a shortcut.

## Testing

Tests assert every panel has open and close commands, filtering is
case-insensitive and matches categories, selection wraps in both directions, and
shortcuts require exact modifiers. The palette rendering test lives in
`alp-ui-kit::overlays`.

## Known limits

- Shortcut hints are static labels; there is no user-editable keymap yet.
- Commands are data only; a command cannot prompt for a parameter or open a
  modal flow yet.
- The palette list has no scrolling; a much larger registry would need it.
