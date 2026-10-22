# `alp-ui-kit` — `theme`

## Purpose

`theme` owns the dark and light glass palettes and the container styles that
paint panels, cards and overlays. No widget or screen defines a literal colour;
everything reads a `ThemeColors` value.

## Public API

- `ThemeColors` fields: background, glass, glass_strong, text, muted, faint,
  accent, accent_hover, accent_pressed, border, border_strong, shadow, success,
  warning, danger, wave, wave_hot, grid, playhead.
- `ThemeColors::DARK` and `LIGHT`.
- `for_mode(ThemeMode)` — resolves system preference at call time.
- `to_theme()` — iced palette for built-in widgets.
- `panel_style(focused)`, `card_style()`, `plain_style()`.

## Data flow

`App::colors()` resolves the stored `ThemeMode` against `system_is_dark()` and
returns a copy. The view passes that value to kit builders, which close over it
in style callbacks. The same value feeds the waveform's grid/wave/playhead
colours and the iced `Theme` used by standard widgets.

Tint alphas are deliberately at least 0.70 for panel surfaces so text remains
legible when the desktop behind is sharp; the window root adds a darker 0.58
scrim.

## Allowed dependencies

`iced` colour, container, border, shadow and theme types; `crate::tokens` for
radii and `ThemeMode`. No assets, no ALP crates.

## Threading / real-time notes

Constants and pure functions; safe to copy into closures every frame. No global
mutable state, so theme switching cannot race with rendering.

## Error handling

No fallible API. An unknown persisted theme string is parsed by `ThemeMode` to
dark. Colours are explicit RGBA values; a caller cannot accidentally ask for a
missing palette.

## Testing

Tests assert the dark/light background and text differ and that glass tint alpha
is at least 0.7 in both themes, which protects the no-blur fallback. Token tests
cover theme cycling and parsing.

## Known limits

- The "system" mode is sampled when the app boots or cycles the theme; it does
  not subscribe to desktop colour-scheme changes.
- There is no high-contrast theme or user-editable accent yet.
- Blur strength cannot be controlled by the app; only tint opacity is ours.
- Colour contrast has not been audited against WCAG text thresholds for every
  state.
