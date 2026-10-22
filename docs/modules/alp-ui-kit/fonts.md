# `alp-ui-kit` — `fonts`

## Purpose

`fonts` bundles one Inter variable font with the binary and exposes the font
handles every widget uses. Bundling removes dependence on a system package and
makes the "all text renders in Inter" acceptance check meaningful.

## Public API

- `INTER_BYTES` — `include_bytes!` of `assets/fonts/InterVariable.ttf`.
- `FAMILY` — the exact family name, `"Inter Variable"`.
- `regular()`, `medium()`, `semibold()`, `bold()` returning `iced::Font` with the
  matching weight.

The OFL text is shipped next to the font as `LICENSE-Inter.txt` and listed in
`THIRD_PARTY.md`.

## Data flow

The crate compiles the font bytes into the binary. `alp-ui::main` registers
`INTER_BYTES` with the iced application builder and sets `fonts::regular()` as
the default. Every kit component and canvas text asks for one of the helpers, so
weight selection does not depend on widget defaults.

The actual family name was verified with `fc-scan`; the variable font reports
`Inter Variable`, not `Inter`. Using the wrong name silently falls back to a
system font, which is why the constant is tested.

## Allowed dependencies

`iced` font types only.

## Threading / real-time notes

Font bytes are static and registered once during application startup. Text
shaping is performed by iced/cosmic-text on the UI thread; no font work happens
on the audio thread.

## Error handling

Registration happens in the iced builder and has no recoverable error in this
API. A missing or malformed asset fails compilation or startup rather than
falling back silently. The unit test asserts the asset is non-trivial in size
and the family/weight values are what the UI requests.

## Testing

`bundled_font_is_present_and_named` checks the byte length is greater than
100 KB and that `regular()` requests `Family::Name("Inter Variable")` with normal
weight. Rendering itself is checked manually with a screenshot.

## Known limits

- Only the variable upright face is bundled; italic is synthetic or absent.
- Glyphs not present in Inter (some symbol/emoji codepoints) fall back to system
  fonts, so not every glyph on screen is Inter.
- The variable font's optical-size axis is not adjusted per text size.
- Users cannot choose a different UI font yet.
