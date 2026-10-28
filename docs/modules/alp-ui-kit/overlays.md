# `alp-ui-kit` — `overlays`

## Purpose

`overlays` builds the floating surfaces above the workspace layer: raised cards,
a modal dialog, a transient toast, the command palette, property rows and empty
states. They are pure builders; the application owns visibility and state.

## Public API

- `CommandItem<Message> { title, hint, message }`.
- `card(content, colors)` — raised inner surface.
- `dialog(title, subtitle, body, footer, width, colors, on_close)` — dimmed,
  centered panel with a title row, a close glyph, a divider and a footer.
- `toast(message, colors, on_dismiss)`.
- `command_palette(id, query, items, selected, colors, on_input)` — the caller
  supplies already-filtered items and an iced widget id for focus.
- `property_row(label, control, colors)` and `empty_state(title, hint, colors)`.

## Data flow

`App::view` pushes layers in order: frame/backdrop, palette, toast, shortcuts,
Create Voice, consent and the first-run wizard. The palette's text input uses
the id from `App`, so opening the overlay can return
`iced::widget::operation::focus::<Message>(id)`; typed text updates
`PaletteState`, and row buttons carry `RunCommand`. A toast carries a dismiss
message; `Message::DismissToast` clears it.

The palette paints a 620 px `panel_style(true)` card with a "Commands" title, a
result chip, a scrolling command list (324 px), and a keyboard hint line. The
dialog uses the same panel style, a divider above the footer and a 14 px radius.

## Allowed dependencies

`iced` widgets, `crate::components`, `crate::fonts`, `crate::theme` and
`crate::tokens`. No state, no ALP crates.

## Threading / real-time notes

View construction only, on the UI thread. The palette can be up to the registry
size; filtering happens in `commands`, not here. No overlay starts a worker or a
timer.

## Error handling

The builders are infallible. A palette with zero items renders "No matching
command." instead of an empty box; the caller's selected index is ignored by the
row loop. A toast has an explicit dismiss action, so it does not depend on a
timer.

## Testing

Tests build a palette with rows and input, a dialog and a toast. Visual placement
and keyboard focus are exercised in the shell, not in unit tests.

## Known limits

- The palette is fixed at 620 logical pixels; a narrow window will clip it.
- There is no focus trap or screen-reader role beyond the text input.
- The dialog always uses primary-styled footer actions; a danger action is not
  distinguished yet.
- Toast lifetime is manual (the user dismisses it); there is no auto-hide timer.
