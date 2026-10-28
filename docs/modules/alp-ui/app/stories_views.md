# `alp-ui` / `app/stories_views`

## Purpose

Renders the Stories workspace: the story list and composer on the left, the
line cards and render controls on the right, and the docked track editor below.
A "Mixer" toggle overlays the shared mixer, inspector and timeline cards.

## Public API

- `App::stories_workspace(&self, colors: ThemeColors) -> Element<'_, Message>`
- Private helpers `story_list`, `story_detail`, `stories_tools_overlay`.

## Data flow

1. Reads `stories`, `selected_story`, `selected_story_line`, `story_text`,
   `profiles`, `profile_name`, `generating`, `stories_tools_open` and the live
   `EditSession`.
2. Emits `SelectStory`, `NewStory`, `SaveStory`, `SelectStoryLine`,
   `DeleteStoryLine`, `StoryLineInput`, `AddStoryLine`, `RenderStory`,
   `SelectVoice`, `ToggleStoriesTools` and the standard transport/editing
   messages used by the embedded panels.
3. Story rendering is performed by `studio.rs`; the view only shows per-line
   status. Both columns scroll through `App::scroll`.

## Allowed dependencies

- `alp-voice` types (`LineStatus`)
- `iced::widget` primitives, `alp-ui-kit` components
- `super::{App, Message}`, `super::workspace_layout::{card, card_auto, header}`

## Threading / real-time notes

Pure view. The track editor and the tools overlay read engine position/meters
through existing views; no audio work happens here.

## Error handling

An empty story list or empty line list renders a muted empty state. Failed line
statuses stay visible after a render so the user can edit and retry.

## Testing

Compile-checked with the shell; behaviour is exercised by the story path in
`studio.rs` and the Stories smoke run.

## Known limits

- Story titles cannot be renamed from the UI yet.
- The tools overlay is a right-aligned `stack` layer; it covers the line cards
  while open and closes through the same Mixer toggle.
- iced rows cannot wrap, so long translations rely on the composer's text area
  rather than line-card wrapping.
