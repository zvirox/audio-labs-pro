# `alp-ui-kit::tokens`

## Purpose

Defines the numeric design tokens used by every screen: spacing, radii, motion
durations, type sizes and theme mode. Values are unitless logical pixels or
milliseconds so no screen hard-codes its own scale.

## Public API

- `Spacing::{XXS, XS, SM, MD, LG, XL, XXL}`
- `Radius::{SM, MD, LG, PILL}`
- `Duration::{FAST_MS, BASE_MS, PANEL_MS}`
- `Motion { reduce_motion }` with `duration_ms(value)`
- `ThemeMode::{Dark, Light, System}`
- `TypeScale::{CAPTION, BODY, BODY_STRONG, TITLE, DISPLAY}`

## Data flow

Screens read a constant and pass it to a widget. `Motion::duration_ms` returns
zero when reduce-motion is enabled, so callers do not branch on the setting.

## Allowed dependencies

None. Pure constants and plain structs; no iced dependency in Phase 1.

## Threading / real-time notes

`Copy` values with no global state. UI thread only; no audio interaction.

## Error handling

Infallible. Theme parsing is the caller’s responsibility.

## Testing

One test verifies reduce-motion collapses a panel transition to zero
milliseconds.

## Known limits

- Logical pixels only; no DPI scaling helper yet.
- The type scale does not carry font weights or line heights.
- No colour palette yet.
