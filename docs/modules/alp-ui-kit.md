# `alp-ui-kit`

## Purpose

One source of truth for visual tokens: type scale, spacing, radii, motion
durations and light/dark themes. Screens and components consume tokens instead
of hard-coded values so the Phase 2 glass shell and later panels stay coherent.

## Public API

- `tokens::ThemeMode`, `TypeScale`, `Spacing`, `Radius`, `Duration`, `Motion`
- crate-level re-exports of the same names

## Data flow

A screen reads a token value (for example `Spacing::MD`) and passes it to an
iced widget. Tokens are plain data with no global state, so they can be used in
tests and previews without a window.

## Allowed dependencies

None on other ALP crates. Phase 1 has no iced dependency; components arrive in
Phase 2 and will use iced plus small presentation crates only.

## Threading / real-time notes

Tokens are immutable constants and `Copy` values. They are used only on the UI
thread and have no audio-thread interaction.

## Error handling

Token constructors are infallible. Invalid theme names are handled by the
caller when parsing configuration.

## Testing

Tests verify that reduce-motion collapses a transition to zero milliseconds.
Token values are constants, so there is no construction logic to test.

## Known limits

- Only tokens exist; there are no widgets or components yet.
- Typography uses logical sizes, not the bundled Inter font metrics.
