# `alp-ui` / `app/track_editor`

## Purpose

Draws the Stories dock: a transport strip, a time ruler, a track gutter and one
lane of accent-coloured clip blocks. It replaces the generic player and mixer strip while
the user is shaping a story, and keeps the timeline visible under the script.

## Public API

- `App::track_editor_view(&self, colors: ThemeColors) -> Element<'_, Message>`
- Private helpers `ruler`, `track_lane`, `clip_style`, `portion`, `shorten`.
- Constants `LANE_HEIGHT` (44 px) and `GUTTER_WIDTH` (44 px).

## Data flow

1. Reads `playing`, `position_secs`, `session.tracks()`, the selected clip and
   `session.duration_secs()`.
2. Emits `PlayPause`, `Stop`, `SplitSelection`, `CopySelection`,
   `ToggleLoopPlayback`, `SelectClip`, `DuplicateClip`, `DeleteClip`,
   `ToggleMixer`, `ToggleInspector` and `ToggleTimeline`.
3. Clip widths use `Length::FillPortion(portion(duration, total_secs))`, and
   gaps use the same weighting, so blocks stay proportional to the story.

## Allowed dependencies

- `alp-ui-kit` components, fonts, tokens and theme
- `iced` widget primitives, including `mouse_area` for clip rows
- `super::{App, Message, format_time}`

## Threading / real-time notes

Pure view. It samples engine position and peaks from the session each frame;
playback and seeking stay on the engine bridge. There is no audio work here.

## Error handling

An empty project renders an empty lane with the ruler only. Clips shorter than
one portion still receive a minimum portion of 1, so `portion` never returns
zero and the row cannot collapse.

## Testing

Unit tests cover `portion` proportionality and minimum width and `shorten`
(char-boundary safe truncation). The dock is compile-checked with the shell and
screenshotted by the Stories smoke run.

## Known limits

- iced has no absolute positioning, so blocks are approximate and cannot be
  dragged horizontally yet; selection and inspector edits are the supported path.
- Only the selected track is drawn as a lane; the track gutter lists the active
  track number.
- Zoom is fixed to the full story duration.
- Waveforms inside clips are flat bars, not peak pyramids.
