# `alp-ui-kit` — `icons`

## Purpose

`icons` is the application's icon vocabulary: compile-time Lucide glyphs
replacing the Unicode punctuation the interface used to draw. Every control
that needs a symbol names a constant from here, which keeps stroke weight and
optical size consistent in dark and light mode at any size.

## Public API

- `Glyph` and `Icon` re-exports from `iced-cube`, plus `themed(glyph, size,
  line_height, tint: Fn(&Theme) -> Color)` — renders an SVG icon a tint
  resolved from the active theme on every draw — and `tinted(glyph, size,
  Option<Color>)` for known-up-front colours.
- About seventy constants across the studio language: transport (`PLAY`,
  `PAUSE`, `STOP`, `RECORD`, `SKIP_BACK`, `SKIP_FORWARD`, `LOOP`, `VOLUME`),
  workspace identities (`GENERATE`, `VOICES`, `CAPTURES`, `STORIES`, `EFFECTS`,
  `MODELS`, `MIXER`, `SETTINGS`, `AI`, `WAVEFORM`, `SIGMA`),
  editing (`TRIM`, `DELETE_KEY`, `TRASH`, `COPY`, `RESET`, `MINUS`, `PENCIL`),
  chrome (`CLOSE`, `CHECK`, `PLUS`, `SEARCH`, `MORE`, `TARGET`, `EXPAND`,
  `COLLAPSE`, `KEYBOARD`), direction (`ARROW_*`, `CHEVRON_*`), files and
  system (`UPLOAD`, `DOWNLOAD`, `OPEN`, `FOLDER`, `STORAGE`, `SAVE`, `SHARE`),
  and feedback (`INFO`, `WARNING`, `SUCCESS`, `FAILURE`, `CLOCK`, `HEART`,
  `EYE`, `STAR`, `LAYERS`, `FILE_TEXT`, …).

## Data flow

Constants expand to `iced_cube::lucide!(Name)` at compile time, embedding the
Lucide SVG path in the binary; only named icons ship. Render helpers wrap the
glyph in iced's `svg` widget and tint it through a closure against
`iced::Theme`, so a single theme switch (dark ⇄ light) recolours every icon.
Buttons, cards and the navigation rail pass `Glyph` values from this module
through `components::button_icon`, `glyph_button*`, `icon_view` and
`icons::tinted`.

## Allowed dependencies

`iced-cube` (which embeds `lucide-static-svg`) and nothing else — no assets
directory, no ALP crates.

## Threading / real-time notes

Constants are `static` item-like values; the SVG handle is created per render
and cached by iced's renderer. Tint closures run on the UI thread each frame
and only read the theme.

## Error handling

There is no fallible path: a glyph name that does not exist in the vendored
Lucide release fails to compile. Unknown names are caught at kit build time,
not at runtime.

## Testing

The kit's component tests construct buttons, cards and rail targets with
several constants, which exercises the `lucide!` expansion and the
`themed`/`tinted` helpers against the pinned iced-cube version.

## Known limits

- Only icons named in this module exist; adding a glyph means adding one
  constant (Lucide names are PascalCase and occasionally renamed upstream —
  `Trash`, not `Trash2`).
- Glyphs are monochrome strokes; two-colour or filled brand icons are out of
  scope.
- Icon tint on button hover is not reactive (see `components` limits).
