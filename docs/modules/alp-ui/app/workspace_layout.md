# `alp-ui` / `app/workspace_layout`

## Purpose

Defines the shell chrome around workspaces: the seven-tab navigation rail, the
docked player, shared card/header/badge helpers, and dispatch to each workspace
view.

## Public API

- `WorkflowTab::{ALL, title, glyph}` with `Generate`, `Stories`, `Captures`,
  `Voices`, `Effects`, `Models`, `Settings`
- `App::sidebar_view`, `App::docked_player_view`, `App::workspace_view`
- Helpers `card`, `header`, `metric_badge`

## Data flow

1. `workflow_tab` selects one workspace; `workspace_view` dispatches to
   `generate_workspace`, `stories_workspace`, `captures_workspace`,
   `voices_workspace`, `effects_workspace`, `models_workspace` or
   `settings_workspace`.
2. The rail emits `SelectWorkflowTab`; the player emits `PlayPause`/`Stop`.
3. Only reading `App` state and constructing messages; no mutations.

## Allowed dependencies

- `alp-ui-kit` components, fonts, tokens and theme
- `iced` widget primitives
- workspace view modules inside `alp-ui`

## Threading / real-time notes

Pure view code. The player reflects engine position updated by `Message::Frame`.

## Error handling

None required; all branches return an element. Status strings fall back to
"Untitled" style defaults when a profile or story is missing.

## Testing

Layout persistence is covered by `workspace.rs` tests; the rail and dispatch are
compile-checked and exercised by the autoclose smoke run.

## Known limits

- The active tab is not persisted across launches yet.
- The rail is fixed at 68 px and has no collapsed mode.
