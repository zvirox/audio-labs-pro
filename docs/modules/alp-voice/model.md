# `alp-voice` / `model`

## Purpose

Defines model identifiers, manifest schema, file metadata, licensing records, status enums, and the standard model catalogue.

## Public API

- `ModelId`: model identifier wrapper
- `ModelKind`: `TtsLight`, `TtsMedium`, `Stt`, `VoiceClone`
- `ModelLicense`: `name`, `spdx`, `terms`, `accepted`
- `ModelFile`: `filename`, `size_bytes`, `sha256`, `url`
- `ModelManifest`: `id`, `name`, `description`, `kind`, `author`, `version`, `license`, `files`, `is_builtin`, `voice_names`, `total_size_bytes`, `is_installed_at`
- `ModelStatus`: `NotInstalled`, `Downloading`, `Installed`, `Verified`, `Corrupt`, `is_ready`
- `standard_catalogue() -> Vec<ModelManifest>`

## Data flow

1. `standard_catalogue` provides definitions for built-in and downloadable models (Kokoro-82M, Kitten Nano).
2. Manifests provide size, voice listings, and verification hashes used by `ModelManager`.

## Allowed dependencies

- `serde::{Deserialize, Serialize}`
- Standard library

## Threading / real-time notes

Data definitions only. Thread-safe and serializable.

## Error handling

Immutable metadata structs; validation is performed at boundary access.

## Testing

Tested as part of `ModelManager` catalogue queries and round-trip tests in `manager.rs`.

## Known limits

- Only English-language models are registered for v0.1 release scope.
