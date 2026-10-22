# 0007 — Component library and motion crate (D7, D8)

- Status: accepted
- Date: 2026-10-22
- Spike: `spikes/ui-iced`

## Context

The plan requires one component API in `alp-ui-kit`, no raw iced buttons in
screens, motion tokens (120/200/320 ms), reduce-motion, and no idle redraws.
Candidates were `iced-plus` 0.1.0, `iced_aw` 0.14.1 and `iced_ui` 0.1.9;
animation candidates `lilt` 0.8.2 and `iced_anim` 0.3.1.

## Findings

| Crate | iced version | License | Adoption risk |
|---|---|---|---|
| `iced_plus` 0.1.0 | not aligned with the iced 0.14 tree (no matching `iced` dep) | MIT | pre-release toolkit; API churn |
| `iced_aw` 0.14.1 | `iced 0.14.0` | MIT | widgets only, not a design system; useful for complex widgets |
| `iced_ui` 0.1.9 | `iced 0.14` | MIT | styling opinions differ from our tokens |
| `lilt` 0.8.2 | toolkit-agnostic | MIT | no spring easing built in |
| `iced_anim` 0.3.1 | iced-specific | MIT | ties kit state to iced widget internals |

## Decision

- Build **`alp-ui-kit` in-house**: tokens, Inter, components, motion. Screens use
  kit components only.
- Adopt `iced_aw` only as an implementation detail behind kit wrappers if a
  widget (e.g. table) would take >2 days to write; never expose its types in
  screens.
- Motion: **`lilt`** for time-based transitions, plus a ~40-line critically
  damped spring in `alp-ui-kit::motion` for panel open/close settle. Tokens:
  120 ms fast, 200 ms base, 320 ms panel; ease-out default.
- Reduce motion sets transitions to instantaneous and disables the spring.

## Evidence

- lilt transition of a scalar at 60 fps adds no measurable cost relative to the
  waveform canvas (17% CPU scrolling vs 0–1% idle).
- `window::frames()` must be subscribed only while animating, otherwise the app
  redraws at 60 fps forever (measured with a minimal iced window).

## Consequences

- iced upgrades are our responsibility; one wrapper module isolates them.
- A clippy `disallowed-methods` list blocks raw `iced::widget::button` in
  `alp-ui` from Phase 2.
