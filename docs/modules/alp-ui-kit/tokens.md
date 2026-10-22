# `alp-ui-kit::tokens`

## Purpose

Defines the numeric design tokens used by every screen: spacing, control sizes,
radii, motion durations, type sizes, theme modes and the waveform display limit.
Values are unitless logical pixels or milliseconds so screens do not invent
their own scale.

## Public API

- `Spacing::{XXS, XS, SM, MD, LG, XL, XXL}`
- `Radius::{SM, MD, LG, PILL}`
- `Control::{SM, MD, LG, GRIP, TITLE_BAR, CHROME}`
- `Duration::{FAST_MS, BASE_MS, PANEL_MS}`
- `Motion { reduce_motion }` with `duration_ms(value)`
- `ThemeMode::{Dark, Light, System}` with `next`, `resolve`, `as_str`, `parse`
- `TypeScale::{CAPTION, BODY, BODY_STRONG, TITLE, DISPLAY}`
- `system_is_dark()` — reads `ALP_SYSTEM_THEME`, `GTK_THEME` or `COLORFGBG`
  and falls back to dark

## Data flow

Screens read a constant and pass it to a widget. `Motion::duration_ms` returns
zero when Reduce motion is enabled, so callers do not branch on the setting.
`system_is_dark` is sampled when a theme is resolved or cycled; it is not a
subscription.

## Allowed dependencies

No ALP crate. It uses iced only for the system-preference lookup and plain Rust
types otherwise.

## Threading / real-time notes

`Copy` constants and small plain structs with no global mutable state. Used on
the UI thread during layout and styling. No audio interaction.

## Error handling

Construction is infallible. Theme parsing accepts known strings and maps
anything else to dark; `resolve` maps `System` using the passed preference and
otherwise returns the requested mode.

## Testing

Tests verify Reduce motion collapses a transition to zero milliseconds, theme
modes round-trip and cycle dark → light → system → dark, and unknown theme
strings fall back to dark. Token values are constants with no construction path
to test.

## Known limits

- Logical pixels only; there is no per-monitor scale helper.
- The type scale carries sizes but not line height, letter spacing or font
  weight.
- Control size tokens do not yet include a density mode.
- Colour lives in `theme`, not in this module.
