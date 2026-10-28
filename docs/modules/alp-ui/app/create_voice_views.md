# `alp-ui` / `app/create_voice_views`

## Purpose

Renders the Create Voice dialog, the modal that turns a recording, a dropped
sample or a description into a stored voice profile. It is a pure view over
`App` state; the consent dialog, profile write and backend calls stay in
`studio.rs`.

## Public API

- `App::create_voice_dialog_view(&self, colors: ThemeColors) -> Element<'_, Message>`
- Private helpers `capture_panel`, `hint_panel`, `right_rule`.

## Data flow

1. Reads `create_voice_open`, `create_voice_tab`, `create_voice_reference`,
   `create_voice_consent`, `create_voice_language`, `profile_name`,
   `profile_tags`, `recording` and `record_started`.
2. Emits `CreateVoiceTab`, `CreateVoiceReferenceInput`, `CreateVoiceConsent`,
   `ProfileNameInput`, `ProfileTagsInput`, `CreateVoiceLanguage`,
   `ToggleRecord`, `CreateDesignedVoice`, `CloseCreateVoice` and
   `CreateVoiceSubmit`.
3. `CreateVoiceSubmit` requires the recorded sample and the speaker-permission
   checkbox, then opens the consent dialog; only `ConfirmConsent` writes a
   profile through `ProfileStore`.
4. The dialog is layered by `App::view` after the workspace and before the
   consent modal, so consent always appears on top.

## Allowed dependencies

- `alp-ui-kit` components, overlays, fonts, tokens and theme
- `iced` widget primitives (`Space`, `column`, `container`, `row`, `text`)
- `super::{App, CreateVoiceTab, Message, format_time}`

## Threading / real-time notes

Pure view. The timer is read from `record_started` once per frame while the
dialog is open; recording itself runs in `alp-engine`. No disk or inference work
happens during rendering.

## Error handling

The Upload tab explains that a WAV can be dropped on the window but no file
picker is bundled; the System Audio tab states that system capture is not wired.
A missing reference recording is reported by the consent dialog rather than
blocking the dialog from opening. Recording stops at 30 seconds.

## Testing

Compile-checked with the shell and covered by `scripts/check-ui.sh`. The submit
and consent paths are exercised in `studio.rs` tests and the
`ALP_START_OVERLAY=create-voice` smoke run.

## Known limits

- Upload cannot open a file chooser; `rfd` is not a dependency.
- System-audio capture is a future backend, not a hidden toggle.
- The reference transcript is stored but not yet used for alignment.
- The waveform bars are a deterministic animated preview, not live input audio.
