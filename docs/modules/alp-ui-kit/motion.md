# `alp-ui-kit` — `motion`

## Purpose

`motion` wraps `lilt` in the two primitives the shell needs: a two-state scalar
toggle and a derived panel offset. It centralizes the Reduce-motion policy so
every caller becomes instantaneous when the user asks for less motion.

## Public API

- `Toggle` with `new(initial)`, `with_duration(ms)`, `set_reduce_motion`,
  `toggle(target)`, `tick(now)`, `value()`, `is_on()` and `is_animating()`.
- `PanelMotion { open, offset }` with `from_open(&Toggle)`.
- `overlay_toggle()` and `panel_toggle()` constructors.
- `scaled(motion, duration)` applies the token policy to a duration.

## Data flow

`App` creates one `Toggle` per panel from the restored open state and a panel
duration token. A frame calls `tick(now)` for every toggle; `App::view` reads
`PanelMotion::from_open` and offsets the panel by up to 14 logical pixels while
it opens. `is_animating()` decides whether the frame subscription stays alive.
Reduce motion sets the duration to zero and calls lilt's instantaneous
transition, so the next `value()` is final.

## Allowed dependencies

`lilt` 0.8.2 and `crate::tokens::{Duration, Motion}`. It does not depend on iced
or any ALP crate.

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
that a panel starts flush when open and gains a 14 px offset after closing.
Token tests cover the 120/200/320 ms scale and reduced-motion duration mapping.

## Known limits

- Panel close animates only position, not opacity or scale.
- Drag settle snaps immediately; there is no spring overshoot or interruptible
  velocity handoff yet, despite the Phase 0 plan mentioning a spring.
- Only panel toggles use the module; buttons and toasts do not animate entrance
  or exit.
- Motion is visual only; it never conveys audio state.
