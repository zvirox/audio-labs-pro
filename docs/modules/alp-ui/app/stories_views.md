# `alp-ui` / `app/stories_views`

## Purpose

Renders the Stories workspace: project list, line editor with per-line status,
render controls, and the shared transport, timeline, mixer and inspector cards.

## Public API

- `App::stories_workspace(&self, colors: ThemeColors) -> Element<'_, Message>`

## Data flow

1. Reads `stories`, `selected_story`, `selected_story_line`, `story_text`,
   `generating`, `voice_status` and the live `EditSession`.
2. Emits `SelectStory`, `NewStory`, `SaveStory`, `SelectStoryLine`,
   `DeleteStoryLine`, `StoryLineInput`, `AddStoryLine`, `RenderStory` and the
   standard transport/editing messages used by the embedded panels.
3. Story rendering is performed by `studio.rs`; the view only shows status.

## Allowed dependencies

- `alp-voice` types (`LineStatus`)
- `iced::widget` primitives, `alp-ui-kit` components
- `super::{App, Message}`, `super::workspace_layout::{card, header}`

## Threading / real-time notes

Pure view. The timeline and mixer read engine meters through existing panel
views; no audio work happens here.

## Error handling

An empty story list or empty line list renders a muted empty state. Failed line
statuses stay visible after a render so the user can edit and retry.

## Testing

Compile-checked with the shell; behaviour is exercised by the story path in
`studio.rs` and the workspace smoke run.

## Known limits

- Story titles cannot be renamed from the UI yet.
- The bottom strip is a fixed 330 px tall, so very dense timelines scroll
  inside their card.
