# `alp-voice` / `profile`

## Purpose

Manages the persistent voice profile library (`~/.local/share/alp/voices`), prebuilt voices ("Aura", "Atlas", "Echo", "Sol"), human consent attestation records for voice cloning, and `.alpvoice` archive import/export.

## Public API

- `VoiceParams`: `speed`, `pitch`, `energy`, `emotion`
- `ConsentRecord`: `new`, `speaker_name`, `attested_by`, `timestamp`, `statement`
- `VoiceProfile`: `builtin`, fields
- `ProfileStore`: `default_store`, `new`, `list_profiles`, `get_profile`, `save_profile`, `delete_profile`, `export_bundle`, `import_bundle`

## Data flow

1. Profiles are stored in XDG data directories (`voices/<id>/profile.json`, `reference.wav`, `embedding.bin`).
2. When creating or importing a cloned profile, `ProfileStore` verifies the presence of an authentic human `ConsentRecord`.
3. `export_bundle` packages profile metadata, audio, and embeddings into `.alpvoice` ZIP archives.
4. `import_bundle` extracts `.alpvoice` archives into the user's local voice profile library.

## Allowed dependencies

- `alp-core::{AppDirs, CoreError}`
- `alp-voice::{VoiceKind, ZipReader, ZipWriter}`
- `serde`, `serde_json`, Standard library

## Threading / real-time notes

File I/O and archive operations run exclusively on background tasks or CLI commands.

## Error handling

Returns `CoreError::Validation` if a cloned profile lacks consent or if bundle archives are corrupt, and `CoreError::Io` on disk errors.

## Testing

Unit tests in `profile.rs` test prebuilt profile seeding, consent requirement enforcement, and bundle export/import round-tripping.

## Known limits

- Library is local to the machine; sync and cloud sharing are outside v1 scope.
