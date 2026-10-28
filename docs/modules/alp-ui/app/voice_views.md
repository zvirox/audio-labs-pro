# `alp-ui` / `app/voice_views`

## Purpose

Renders the Voices workspace: the profile library grid, the selected-profile
inspector, the cloning gate and the profile storage card. Import Voice and
Create Voice actions live in the page header above both columns.

## Public API

- `App::voices_workspace(&self, colors: ThemeColors) -> Element<'_, Message>`
- Private helper `voice_inspector`.

## Data flow

1. Reads `profiles` via `profile_grid(4, colors)`, `selected_profile`,
   `profile_store`, `hardware_summary` and the derived consent state.
2. Emits `SelectProfile`, `ExportProfile`, `DeleteProfile`,
   `OpenCreateVoice`, `ImportVoice`, `OpenConsentDialog`,
   `AuditReferenceAudio` and `SelectProfile` (Use in Generate).
3. Library and inspector columns both scroll through `App::scroll`; design and
   clone creation run in `studio.rs`, so the view never touches disk.

## Allowed dependencies

- `alp-voice::{VoiceKind, VoiceParams}` types
- `alp-ui-kit` components/tokens, `iced` widget primitives
- `super::{App, Message}`, `super::workspace_layout::{card_auto, header,
  metric_badge}`

## Threading / real-time notes

Pure view. Recording and audit belong to the engine and `studio.rs`; this
workspace only displays the result and a consent chip.

## Error handling

An empty library keeps the grid empty; a missing selection shows "Select a
profile to inspect it." A missing profile store renders "Profile store
unavailable". Consent missing on a cloned profile is shown as a chip rather than
hiding the profile.

## Testing

Compile-checked; profile save/delete/export/import are covered by `alp-voice`
profile tests and the UI smoke run.

## Known limits

- Bundle import is drop-only: `.alpvoice` files are routed by `ImportPath`.
- Clone last take always points at the most recent recording.
- The inspector shows the profile's default speed/pitch but does not edit them.
