# `alp-ui` / `app/generate_views`

## Purpose

Renders the Generate workspace: the voice profile grid, the Generate Speech
card (voice picker, prompt editor, generate/play/status) and the take history
panel. Import Voice and Create Voice actions sit in the page header.

## Public API

- `App::generate_workspace(&self, colors: ThemeColors) -> Element<'_, Message>`
- Private helpers `generate_box`, `history_panel`, `history_row`.

## Data flow

1. Reads the profile library through `profile_grid(3, colors)`, plus
   `voice_editor`, `voice_status`, `generating` and `history`.
2. Emits `GenerateEditor` (text-editor action), `GenerateSpeech`,
   `SelectVoice`, `PlayHistory`, `InsertHistory`, `DeleteHistory`,
   `ClearHistory`, `ImportVoice` and `OpenCreateVoice`.
3. All rendering is pure; generation, playback and insertion are handled in
   `studio.rs`. The left column scrolls through `App::scroll`.

## Allowed dependencies

- `iced::widget` primitives, `alp-ui-kit` components/fonts/tokens
- `super::{App, Message, format_time}`, `super::workspace_layout::header`

## Threading / real-time notes

None. The view never synthesizes or decodes; it only reflects worker state and
the status line. The prompt editor is an `iced` text editor whose `Action`
message is forwarded.

## Error handling

The history panel shows an empty state until a take exists. Generate is a
primary button that stays visible while working; the status line reports worker
progress and failures. Playback errors surface as toasts from the handler.

## Testing

Exercised by the UI smoke run and `scripts/check-ui.sh`; widget construction is
compile-checked with the rest of `alp-ui`.

## Known limits

- `Play last` targets history index 0 even when the list is empty; the handler
  guards the case and shows no toast.
- History rows are truncated to 110 characters for display only.
- The profile grid is three columns inside the 11/20-width left column; a fourth
  profile starts a new row.
