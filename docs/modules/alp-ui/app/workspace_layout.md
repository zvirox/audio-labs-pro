# `alp-ui` / `app/workspace_layout`

## Purpose

Defines the shell chrome around workspaces: the rounded application frame, the
34 px title strip, the 80 px icon rail, the docked player or Stories track
editor, the shared card/header/badge/profile helpers and dispatch to each
workspace view.

## Public API

- `WorkflowTab::{ALL, title, glyph, from_slug}` with `Generate`, `Stories`,
  `Captures`, `Voices`, `Effects`, `Models`, `Settings`.
- `App::sidebar_view`, `App::docked_player_view`, `App::dock_view`,
  `App::workspace_view`, `App::traffic_lights`.
- `App::scroll(content, colors)` — every scrolling surface goes through it.
- `App::profile_card(profile, colors)` and `App::profile_grid(columns, colors)`.
- Helpers `card`, `card_auto`, `header`, `metric_badge`.

## Data flow

1. `workflow_tab` selects one workspace; `workspace_view` dispatches to
   `generate_workspace`, `stories_workspace`, `captures_workspace`,
   `voices_workspace`, `effects_workspace`, `models_workspace` or
   `settings_workspace`.
2. The rail emits `SelectWorkflowTab`; the player emits `PlayPause`/`Stop`.
3. `dock_view` picks the player for every tab except Stories, which docks the
   track editor with its own transport messages.
4. `profile_grid` lays cards in equal `FillPortion(1)` slots and fills a short
   last row with weighted spacers, so a partial row keeps column width.
5. Only reading `App` state and constructing messages; no mutations.

## Allowed dependencies

- `alp-ui-kit` components, fonts, tokens and theme
- `iced` widget primitives
- workspace view modules inside `alp-ui`

## Threading / real-time notes

Pure view code. The player reflects engine position updated by `Message::Frame`.
Scroll styling is stateless and rebuilt per frame.

## Error handling

None required; all branches return an element. Status strings fall back to
"Untitled" style defaults when a profile or story is missing.

## Testing

Layout persistence is covered by `workspace.rs` tests; the rail, grid and
dispatch are compile-checked and exercised by the autoclose smoke run.

## Known limits

- The active tab is not persisted across launches yet.
- The rail is fixed at 80 px and has no collapsed mode.
- The frame is rounded with no client-side shadows on Wayland, where the
  compositor decides whether to draw one.
