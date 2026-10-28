# `alp-ui-kit::tokens`

## Purpose

Defines the numeric design tokens used by every screen: spacing, control sizes,
rail geometry, radii, motion durations, type sizes, theme modes and the waveform
display limit. Values are unitless logical pixels or milliseconds so screens do
not invent their own scale.

## Public API

- `Spacing::{XXS, XS, SM, MD, LG, XL, XXL}` = 2/4/8/12/16/24/32.
- `Radius::{SM, MD, LG, XL, PILL}` = 6/10/14/18/999.
- `Control::{SM, MD, LG, PAGE, TITLE_BAR, DOCK, GRIP, CHROME}` = 28/36/44/32/
  34/96/14/34.
- `Rail::{WIDTH, BUTTON, LOGO}` = 80/44/48.
- `Duration::{FAST_MS, BASE_MS, PANEL_MS}` = 120/200/320.
- `Motion { reduce_motion }` with `duration_ms(value)`.
- `ThemeMode::{Dark, Light, System}` with `next`, `resolve`, `as_str`, `parse`.
- `TypeScale::{CAPTION, LABEL, BODY, BODY_STRONG, HEADING, TITLE, DISPLAY}` =
  12/13/14/15/17/22/30.
- `system_is_dark()` — reads `ALP_SYSTEM_THEME`, `GTK_THEME` or `COLORFGBG`
  and falls back to dark.

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
