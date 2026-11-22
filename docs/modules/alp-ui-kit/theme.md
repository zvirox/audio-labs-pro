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

`App::colors()` resolves the stored `ThemeMode` against `system_is_dark()`,
applies the active workspace's [`Neon`] accent through `with_neon()` and returns a
copy. The view passes that value to kit builders, which close over it in style
callbacks. The same value feeds the waveform's grid/wave/playhead colours and
`to_theme()`, which maps it onto an iced-cube `Config`: the workspace accent becomes
the cube `primary`, raised surfaces become its secondary and hover accent,
danger/success/warning pass through, and the studio hairline becomes the cube
border. Dark or light is chosen by background luminance, and derived cube
tokens (foreground-on-primary and friends) are computed from that mapping, so
switches, checkboxes and buttons recolour on the same palette switch that the
workspace or theme flip produces.

The dark palette uses calm slate surfaces (background 0.043, glass 0.067, cards
0.086, inputs 0.114) with text at 0.945/0.667/0.518 and restrained workspace
accents whose ink is derived by iced-cube's luminance rule. Borders are a
low-alpha slate tint and the accent border uses the active hue. The light palette
mirrors the same roles on paper surfaces with deepened accents. Each shell
workspace owns one hue — Generate cyan, Stories magenta, Captures green, Voices
violet, Effects orange, Models blue, Settings amber — and the kit default (also
the boot workspace) is cyan. The window root paints the background colour behind
the 18 px frame; there is no separate alpha scrim in the frame.

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

Tests assert the dark/light background and text differ, that the two palettes
already carry the default cyan hue, that `with_neon()` is idempotent for the
default hue, that all seven hues stay distinct per mode with ordered
hover/pressed/hot variants, that ink follows the cube luminance rule, and that
every text role and accent ink meets WCAG AA on every surface in both themes.
Token tests cover theme cycling and parsing.

## Known limits

- The "system" mode is sampled when the app boots or cycles the theme; it does
  not subscribe to desktop colour-scheme changes.
- There is no high-contrast theme or user-editable accent mapping yet (the
  hue set is fixed per workspace in code).
- Blur strength cannot be controlled by the app; only tint opacity is ours.
- Colour contrast is checked by unit tests for text/muted/faint roles and
  accent ink, but not for every possible translucent overlay combination.
- The iced-cube mapping is fixed: workspace accent → cube primary, raised surface
  → secondary/accent. Cube-derived hovers (dark-theme primary darkens) follow
  iced-cube's emphasis convention rather than a per-token ALP choice.
