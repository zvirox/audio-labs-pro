# `alp-ui-kit` — `theme`

## Purpose

`theme` owns the dark and light studio palettes and the container styles that
paint the frame, rail, panels, cards, wells and overlays. No widget or screen
defines a literal colour; everything reads a `ThemeColors` value.

## Public API

- `ThemeColors` fields: background, glass, glass_strong, surface_raised, text,
  muted, faint, accent, accent_text, accent_hover, accent_pressed,
  accent_secondary, border, border_strong, shadow, scrim, success, warning,
  danger, wave, wave_hot, grid, playhead.
- `ThemeColors::DARK` and `LIGHT`.
- `for_mode(ThemeMode)` — resolves system preference at call time.
- `to_theme()` — builds the iced `Theme` through `iced_cube::theme::Config` so
  both built-in widgets and iced-cube controls resolve the studio palette.
- `frame_style()`, `rail_style()`, `panel_style(focused)`, `card_style()`,
  `well_style()`, `plain_style()`.

## Data flow

`App::colors()` resolves the stored `ThemeMode` against `system_is_dark()` and
returns a copy. The view passes that value to kit builders, which close over it
in style callbacks. The same value feeds the waveform's grid/wave/playhead
colours and `to_theme()`, which maps it onto an iced-cube `Config`: the gold
accent becomes the cube `primary`, raised surfaces become its secondary and
hover accent, danger/success/warning pass through, and the studio hairline
becomes the cube border. Dark or light is chosen by background luminance, and
derived cube tokens (foreground-on-primary and friends) are computed from that
mapping, so switches, checkboxes and buttons recolour on the same theme switch
that flips `ThemeColors`.

The dark palette uses near-black surfaces (background 0.039, glass 0.055, cards
0.078, inputs 0.106), text at 0.96/0.62/0.40 and a warm gold accent
`rgb(0.780, 0.651, 0.294)` with dark ink for filled buttons. Borders are white at
10% and the accent border at 55%. The light palette mirrors the same roles on
paper surfaces with a deeper gold. The window root paints the background colour
behind the 18 px frame; there is no separate alpha scrim in the frame.

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

Tests assert the dark/light background and text differ, that the studio accent
is gold in both modes, and that borders stay translucent. Token tests cover
theme cycling and parsing.

## Known limits

- The "system" mode is sampled when the app boots or cycles the theme; it does
  not subscribe to desktop colour-scheme changes.
- There is no high-contrast theme or user-editable accent yet.
- Blur strength cannot be controlled by the app; only tint opacity is ours.
- Colour contrast has not been audited against WCAG text thresholds for every
  state.
- The iced-cube mapping is fixed: gold → cube primary, raised surface →
  secondary/accent. Cube-derived hovers (dark-theme primary darkens) follow
  iced-cube's emphasis convention rather than a per-token ALP choice.
