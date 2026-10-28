# `alp-ui` / `app/voice_views`

## Purpose

Renders the Voices workspace: the persisted profile library, designed/cloned
voice creation, the reference-sample card and the consent gate summary.

## Public API

- `App::voices_workspace(&self, colors: ThemeColors) -> Element<'_, Message>`

## Data flow

1. Reads `profiles`, `selected_profile`, `profile_name`, `profile_tags`,
   `record_path`, `reference_audit_status`, `recording` and `hardware_summary`.
2. Emits `SelectProfile`, `ExportProfile`, `DeleteProfile`, `ProfileNameInput`,
   `ProfileTagsInput`, `CreateDesignedProfile`, `OpenConsentDialog`,
   `ToggleRecord` and `AuditReferenceAudio`.
3. Design and clone creation run in `studio.rs`; the view never touches disk.

## Allowed dependencies

- `alp-voice::{VoiceKind}` types
- `alp-ui-kit` components/tokens, `iced` widget primitives
- `super::{App, Message}`, `super::workspace_layout::{card, header}`

## Threading / real-time notes

Pure view. Recording itself is owned by the engine; this workspace only starts
and stops it and displays audit results.

## Error handling

Empty tags and missing consent are shown inline. The cloning card states that a
consent record is mandatory and that the built-in backend does not clone timbre.

## Testing

Compile-checked; profile save/delete/export/import are covered by `alp-voice`
profile tests and the UI smoke run.

## Known limits

- Bundle import is drop-only: `.alpvoice` files are routed by `ImportPath`.
- The reference card always points at the most recent recording.
