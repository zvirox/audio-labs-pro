# `alp-ui-kit` — `motion`

## Purpose

`motion` wraps `lilt` in the two primitives the shell needs: a two-state scalar
toggle and a derived panel offset. It centralizes the Reduce-motion policy so
every caller becomes instantaneous when the user asks for less motion.

## Public API

- `Toggle` with `new(initial)`, `with_duration(ms)`, `set_reduce_motion`,
  `toggle(target)`, `tick(now)`, `value()`, `is_on()` and `is_animating()`.
- `PanelMotion { open, offset }` with `from_open(&Toggle)`.
- `offset(content, x, y)` — draws a child with a translation that does not
  change its layout bounds.
- `overlay_toggle()` and `panel_toggle()` constructors.
- `scaled(motion, duration)` applies the token policy to a duration.

## Data flow

`App` creates one `Toggle` per panel from the restored open state and a panel
duration token, plus a one-shot workspace toggle that re-arms on tab changes.
A frame calls `tick(now)` for every toggle; `App::view` reads
`PanelMotion::from_open`, calls `offset` on the workspace content and settles it
by up to 8 logical pixels. `is_animating()` keeps the frame subscription alive
only while a transition is running. Reduce motion sets the duration to zero and
calls lilt's instantaneous transition, so the next `value()` is final.

## Allowed dependencies

`lilt` 0.8.2, `iced` widget/renderer types for the translation wrapper and
`crate::tokens::{Duration, Motion}`. It does not depend on any ALP domain crate.

## Threading / real-time notes

State lives on the UI thread. `lilt` is time-based and does no work when a value
is at target; the app stops subscribing to frames when all toggles are idle.
`value()` uses `Instant::now()` so a render without a tick still reflects elapsed
time.

## Error handling

There are no fallible operations. Durations are `u16` tokens; setting zero is the
defined reduced-motion path. An out-of-range initial value is compared against
0.5 for `on` but the animation clamps its output to 0..=1.

## Testing

Tests prove reduced motion is instantaneous and not reported as animating, and
that content starts flush when open and gains an 8 px offset after closing.
Token tests cover the 120/200/320 ms scale and reduced-motion duration mapping.

## Known limits

- The workspace transition animates position only, not opacity or scale.
- Drag settle snaps immediately; there is no spring overshoot or interruptible
  velocity handoff yet, despite the Phase 0 plan mentioning a spring.
- Buttons use distinct cube hover/pressed fills, but they do not run a timed
  interpolation; toasts and dialogs appear without an entrance transition.
- Motion is visual only; it never conveys audio state.
