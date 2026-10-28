# `alp-ui-kit`

## Purpose

The in-house design system for Audio Labs Pro: design tokens, bundled Inter, the
studio graphite palette in a dark and a light variant, semantic controls, overlay
builders, motion primitives and the peak-pyramid waveform canvas. Screens consume
these modules instead of raw iced widgets, which is how the shell keeps one
visual language and how the no-raw-button rule stays enforceable.

The studio language is deliberately restrained: near-black surfaces, graphite
cards, a single warm gold accent with dark ink on filled buttons, 6/10/14/18 px
radii and pill controls, hairline white borders at 10% and soft shadows. Glyph
buttons render single-character monochrome symbols rather than an icon font.

## Public API

- `tokens` — `Spacing`, `Radius`, `Control`, `Rail`, `TypeScale`, `Motion`,
  `ThemeMode`, `Duration`, `system_is_dark`.
- `fonts` — bundled `INTER_BYTES`, `FAMILY`, `regular`, `medium`, `semibold`,
  `bold`, `mono`.
- `theme` — `ThemeColors` (`DARK`, `LIGHT`), `for_mode`, `to_theme`,
  `frame_style`, `rail_style`, `panel_style`, `card_style`, `well_style`,
  `plain_style`.
- `components` — `ButtonKind`, pill and fill buttons, rail/glyph/compact
  buttons, toggle and switch, sliders, text fields and areas, dropdowns,
  segmented control, tabs, chips, avatar, tooltip, progress, scrolling style and
  the page/card/section/field text helpers.
- `overlays` — cards, dialog, toast, command palette, property row, empty state.
- `motion` — `Toggle`, `PanelMotion`, overlay/panel constructors and duration
  scaling.
- `waveform` — validated peak levels, `WaveformPeaks`, `WaveformError` and the
  scrollable `WaveformWidget`.
- Crate-level re-exports of the commonly used token/theme/component names.

## Data flow

```text
tokens + theme ──► components/overlays ──► App::view
       │                    ▲
       └──► Motion ─────────┘
media pyramid ──► WaveformPeaks ──► WaveformWidget ──► iced canvas
```

Every builder is stateless. The app owns values and messages; the kit turns them
into styled iced elements. The waveform owns immutable peaks and a small view
state (scroll, zoom, playhead, selection).

## Allowed dependencies

Only external presentation crates: `iced` 0.14 (canvas/advanced) and `lilt` 0.8.2.
No ALP crate is allowed. `alp-media` data is converted by `alp-ui` into
`waveform::WaveformPeaks`; the kit never parses a project or peak cache.

## Threading / real-time notes

All APIs run on the UI thread. Peaks are immutable and shared behind `Arc` with
the renderer. Motion is time-based and stops consuming frames when idle.
Components own no timers or threads. None of this code touches the audio
callback.

## Error handling

`WaveformPeaks::new` and `PeakLevel::new` return typed `WaveformError` values for
zero sample rates/frames/buckets, no levels, or unordered levels. Widget and
component constructors are otherwise infallible. Theme parsing falls back to
dark; tooltips and overlays accept empty content without panicking.

## Testing

Unit tests cover token/theme parsing and reduce motion, the studio palette roles
and tint alpha, the bundled font family and size, button/glyph/rail/toggle/
switch/slider/segmented construction, dialog/toast/palette building, panel
motion, and waveform validation, level selection, aggregation and scroll/zoom
clamping. `scripts/check-ui.sh` additionally enforces that
`crates/alp-ui/src` never calls a raw iced button.

## Known limits

- The component set does not yet include a standalone popover or a spinner.
- Icons are text glyphs, not a dedicated icon font.
- Button state changes are immediate; only panels animate.
- The waveform is display-only and supports i16 peaks up to one hour.
- There is no component gallery, visual regression suite or accessibility audit
  beyond contrast-by-design and reduced motion.
