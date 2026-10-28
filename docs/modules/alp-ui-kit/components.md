# `alp-ui-kit` — `components`

## Purpose

`components` is the only place that constructs an iced button. Screens receive
semantic variants and states from here, which keeps padding, radius, colours,
font weight and disabled behaviour consistent. All components are stateless;
the application owns values and messages.

Since the iced-cube adoption, the interactive controls (buttons, switches,
checkboxes) borrow their colours from iced-cube's style functions resolved from
the active `iced::Theme`; the kit keeps its own geometry (pill radius, control
heights, rail targets) and its studio component language.

## Public API

- `ButtonKind { Primary, Secondary, Ghost, Danger, Icon }` and
  `ButtonKind::cube_variant()` mapping.
- `button(label, kind, on_press: Option<Message>)` — pill button, 36 px.
- `button_fill(label, kind, on_press)` — stretches to the row width.
- `button_large(...)` — 44 px primary-height action.
- `button_icon(label, icon: Glyph, kind, on_press)` — labelled pill with a
  leading Lucide icon.
- `icon_button(glyph, on_press)`, `glyph_button(glyph, size, kind, on_press)`
  and `glyph_button_compact(...)` (28 px).
- `rail_button(glyph, active, on_press)` — 44 px circular rail target.
- `toggle(label, value, on_toggle)` and `switch(value, on_toggle)`
  (unlabelled, for dense rows) — rendered by the iced-cube `Switch`.
- `checkbox(label, checked, on_toggle: Fn(bool) -> Message)` — iced-cube
  checkbox for confirmations and selectable options.
- `themed_slider(value, colors, on_change)` and the legacy `slider_control`.
- `text_field(placeholder, value, colors, on_input)` and `text_area(content,
  placeholder, height, colors, on_action)`.
- `dropdown(options, selected, colors, on_select)`.
- `tabs(labels, active, on_select)` and `segmented(labels, active, colors,
  on_select)`.
- `chip(label, colors)`, `avatar(name, diameter, colors)`.
- `icon_view(glyph, size, kind)` — Lucide icon pre-tinted with a variant's
  rest colour.
- `scroll_style(colors)` — transparent rail with a subtle muted scroller.
- `with_tooltip(content, label, colors)`, `progress(value, colors)`.
- `page_title`, `section_title`, `card_title`, `field_label`, `kicker`, `muted`.

## Data flow

Button-like builders map a semantic `ButtonKind` onto an iced-cube variant and
call `iced_cube::primitives::button::style` with `Tokens::of(theme)` inside the
style closure, overriding only the border radius with the studio pill value.
Switch, toggle and checkbox delegate to the iced-cube widgets, which resolve
the same tokens. Token-styled surfaces (sliders, inputs, dropdowns, segmented
tracks, chips, avatars, scroll rails, progress) still close over an explicit
`ThemeColors` so they can be drawn on any panel. `on_press: Option<Message>`
becomes `on_press_maybe`, so a `None` action renders a disabled-looking control
without a second code path. The app sends the resulting message; the kit never
stores application state. `scroll_style` is applied by `App::scroll` so every
workspace shares the same scroller.

### Button geometry contract

iced's raw `Button` lays its content out from the top-left with padding only,
and derives a fresh button's default length from its *content's* `size_hint`.
The kit therefore does two things for every label pill:

- The content is wrapped in a `Fill`/`Fill` container centred on both axes, so
  labels and icons sit mid-pill at any control height (iced-cube's own
  behaviour). Disabled controls dim their icons via
  `iced_cube::icon::opacity`.
- The button length is pinned explicitly (`Shrink` unless the caller passes a
  width): without the pin, the centring container would leak `Fill` into every
  pill, and a row of such pills would split its row width evenly instead of
  hugging their labels.

The pinned `Shrink` button is additionally wrapped in a port of iced-cube's
`natural` shell: a shrink-width widget is measured without an upper width
bound, so when the parent row runs out of room the whole pill overflows
instead of being squeezed until its label wraps or paints past the rounded
ends. Sizes follow iced-cube's mapping onto the studio scale: 36 px pills use
13 px text (cube `Size::Sm`), 44 px pills use 15 px text and larger horizontal
padding (cube `Size::Lg`).

## Allowed dependencies

`iced` widgets, `iced-cube` primitives and theme tokens, and the crate's own
`fonts`, `icons`, `theme` and `tokens` modules. It never depends on an ALP
domain crate, so the kit stays reusable and testable.

## Threading / real-time notes

Builders run on the UI thread while constructing the view. They perform small
string allocations for labels; no state or timer is owned by the kit. Motion is
driven by `crate::motion` in the application loop, not by per-widget threads.

## Error handling

There are no fallible constructors. An empty label, empty option list or `None`
action is valid and produces an inert control. A caller that passes an
out-of-range slider value is clamped by iced's range widget.

## Testing

Tests build labelled buttons with and without a message, icon-labelled pills,
fill/large variants, rail and compact glyph buttons, toggle/switch/checkbox and
slider with a message closure, text fields, tabs, a segmented strip and chips.
The suite also ensures the components compile against the pinned iced and
iced-cube versions, which is the main compatibility risk.

## Known limits

- Glyph buttons cannot change their icon tint on hover: iced `Svg` styling has
  no hover status, so icons keep the variant's rest colour while the pill recolours.
- Variants have immediate hover/pressed colours from iced-cube, not a
  per-button spring.
- There is no loading/spinner button state yet.
- Tooltips are placed above their content; edge-aware placement is not
  implemented.
- A standalone popover is not provided; dialog, toast and command palette cover
  the current overlays.
