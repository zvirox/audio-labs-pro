# `frontend/src/shortcuts`

## Purpose

Global keyboard shortcut interception, Command Palette navigation (`cmdk`), and hotkey reference cheatsheets for Audio Labs Pro.

## Public API

- `useKeyboardShortcuts` — React hook capturing global studio shortcuts (`Ctrl+1..7`, `Ctrl+K`, `F1`, `Ctrl+D`, `Ctrl+Shift+M`, `Ctrl+B`).
- `CommandPalette` — Fast keyboard-driven command runner using `cmdk` for fuzzy search across workspaces, panels, and actions.
- `ShortcutsModal` — Comprehensive modal cheatsheet displaying all active shortcuts grouped by functional domain.

## Data flow

1. Keydown events are filtered to avoid intercepting active text input elements.
2. Recognized chords dispatch corresponding state changes (workspace switch, theme cycle, dialog toggle).
3. The command palette executes selected actions and automatically dismisses itself.

## Allowed dependencies

- `cmdk`
- `frontend/src/kit`
- Lucide React icons

## Threading / real-time notes

Shortcut handling executes synchronously in the webview event loop; action dispatches are non-blocking.

## Error handling

Unhandled key events pass through normally to native webview handlers.

## Testing

Verified with keyboard-only navigation across all seven workspaces and command execution paths.

## Known limits

Browser dev tools hotkeys (`Ctrl+Shift+I` / `F12`) are preserved in development builds and not intercepted.
