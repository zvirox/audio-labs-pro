# `alp-ui-kit` — `components`

## Purpose

`components` is the only place that constructs an iced button. Screens receive
semantic variants and states from here, which keeps padding, radius, colours,
font weight and disabled behaviour consistent. All components are stateless;
the application owns values and messages.

## Public API

- `ButtonKind { Primary, Secondary, Ghost, Danger, Icon }`.
- `button(label, kind, colors, on_press: Option<Message>)` — pill button, 36 px.
- `button_fill(label, kind, colors, on_press)` — stretches to the row width.
- `button_large(...)` — 44 px primary-height action.
- `icon_button(glyph, colors, on_press)`, `glyph_button(glyph, size, kind,
  colors, on_press)` and `glyph_button_compact(...)` (28 px).
- `rail_button(glyph, active, colors, on_press)` — 44 px circular rail target.
- `toggle(label, value, colors, on_toggle)` and `switch(value, colors,
  on_toggle)` (unlabelled, for dense rows).
- `themed_slider(value, colors, on_change)` and the legacy `slider_control`.
- `text_field(placeholder, value, colors, on_input)` and `text_area(content,
  placeholder, height, colors, on_action)`.
- `dropdown(options, selected, colors, on_select)`.
- `segmented(labels, active, colors, on_select)` and `tabs(...)`.
- `chip(label, colors)`, `avatar(name, diameter, colors)`.
- `scroll_style(colors)` — transparent rail with a subtle muted scroller.
- `with_tooltip(content, label, colors)`, `progress(value, colors)`.
- `page_title`, `section_title`, `card_title`, `field_label`, `kicker`, `muted`.

## Data flow

Each builder maps a semantic variant plus `ThemeColors` into an iced widget
style. `on_press: Option<Message>` becomes `on_press_maybe`, so a `None` action
renders a disabled-looking control without a second code path. The app sends the
resulting message; the kit never stores application state. `scroll_style` is
applied by `App::scroll` so every workspace shares the same scroller.

## Allowed dependencies

`iced` widgets and the crate's own `fonts`, `theme` and `tokens` modules. It
never depends on an ALP domain crate, so the kit stays reusable and testable.

## Threading / real-time notes

Builders run on the UI thread while constructing the view. They perform small
string allocations for labels; no state or timer is owned by the kit. Motion is
driven by `crate::motion` in the application loop, not by per-widget threads.

## Error handling

There are no fallible constructors. An empty label, empty option list or `None`
action is valid and produces an inert control. A caller that passes an
out-of-range slider value is clamped by iced's range widget.

## Testing

Tests build labelled buttons with and without a message, fill/large variants,
rail and compact glyph buttons, toggle/switch and slider with a message closure,
text fields, a segmented strip and chips. The suite also ensures the components
compile against the pinned iced version, which is the main compatibility risk.

## Known limits

- Icons are Unicode glyphs rendered in Inter, not an icon font; some symbols may
  fall back to another installed font.
- Variants have immediate hover/pressed colours, not a per-button spring.
- There is no loading/spinner button state yet.
- Tooltips are placed above their content; edge-aware placement is not
  implemented.
- A standalone popover is not provided; dialog, toast and command palette cover
  the current overlays.
