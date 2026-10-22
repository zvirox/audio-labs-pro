# `alp-ui-kit` — `overlays`

## Purpose

`overlays` builds the floating surfaces above the panel layer: raised cards, a
modal dialog, a transient toast, the command palette, property rows and empty
states. They are pure builders; the application owns visibility and state.

## Public API

- `CommandItem<Message> { title, hint, message }`.
- `card(content, colors)` — raised inner surface.
- `modal(title, body, actions, colors)` — dimmed, centered dialog with a primary
  action row.
- `toast(message, colors, on_dismiss)`.
- `command_palette(id, query, items, selected, colors, on_input)` — the caller
  supplies already-filtered items and an iced widget id for focus.
- `property_row(label, control, colors)` and `empty_state(title, hint, colors)`.

## Data flow

`App::view` pushes layers in order: window tint, panels, chrome, palette, toast.
The palette's text input uses the id from `App`, so opening the overlay can
return `iced::widget::operation::focus::<Message>(id)`; typed text updates
`PaletteState`, and row buttons carry `RunCommand`. A toast carries a dismiss
message. The modal is available for future destructive confirmations but is not
currently opened by the shell.

## Allowed dependencies

`iced` widgets, `crate::components`, `crate::fonts`, `crate::theme` and
`crate::tokens`. No state, no ALP crates.

## Threading / real-time notes

View construction only, on the UI thread. The palette can be up to the registry
size; filtering happens in `commands`, not here. No overlay starts a worker or a
timer.

## Error handling

The builders are infallible. A palette with zero items renders only the input
and the help line; the caller's selected index is ignored by the row loop. A
toast has an explicit dismiss action, so it does not depend on a timer.

## Testing

Tests build a palette with rows and input, a modal and a toast. Visual placement
and keyboard focus are exercised in the shell, not in unit tests.

## Known limits

- The palette is fixed at 640 logical pixels and its list does not scroll; a
  narrow window or a very large registry will clip.
- There is no focus trap or screen-reader role beyond the text input.
- The modal always uses primary-styled actions; a danger action is not
  distinguished yet.
- Toast lifetime is manual (the user dismisses it); there is no auto-hide timer.
