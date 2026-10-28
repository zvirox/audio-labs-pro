# `alp-ui` / `app/generate_views`

## Purpose

Renders the Generate workspace: the prompt box, profile picker, render status,
take history with play/insert/delete actions, and an engine/storage summary.

## Public API

- `App::generate_workspace(&self, colors: ThemeColors) -> Element<'_, Message>`

## Data flow

1. Reads `voice_text`, `profiles`, `selected_profile`, `history`, `generating`,
   `voice_status` and `media_root` from `App`.
2. Emits `GenerateTextInput`, `GenerateSpeech`, `SelectVoice`, `PlayHistory`,
   `InsertHistory`, `DeleteHistory` and `ClearHistory`.
3. All rendering is pure; playback and insertion are handled in `studio.rs`.

## Allowed dependencies

- `iced::widget::{Space, column, container, row, scrollable, text, text_input}`
- `alp-ui-kit::{ButtonKind, ThemeColors, components, fonts, tokens}`
- `super::{App, Message, format_time}`, `super::workspace_layout::{card, header}`

## Threading / real-time notes

None. The view never synthesizes or decodes; it only reflects worker state.

## Error handling

Shows an empty-state card when no take exists and disables the generate label
while a render is running. Playback errors surface as toasts from the handler.

## Testing

Exercised by the UI smoke run and `scripts/check-ui.sh`; widget construction is
compile-checked with the rest of `alp-ui`.

## Known limits

- `Play last` targets history index 0 even when the list is empty; the handler
  guards the case and shows no toast.
- History rows are truncated to 110 characters for display only.
