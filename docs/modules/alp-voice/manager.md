# `alp-voice` / `manager`

## Purpose

Manages the local on-disk catalogue of AI models, handling license acceptance, disk space checks, file installation, SHA-256 verification, and offline mode.

## Public API

- `ModelManagerConfig`: `storage_dir`, `offline_mode`
- `ModelManager`: `new`, `with_config`, `storage_dir`, `model_dir`, `is_offline`, `set_offline`, `catalogue`, `get_manifest`, `accept_license`, `status`, `verify`, `install_file`, `remove`

## Data flow

1. Resolves model storage paths within standard XDG data directories (`$XDG_DATA_HOME/alp/models`).
2. `install_file` writes incoming downloads to temporary `.part` files before atomic rename.
3. `verify` reads on-disk files and computes their SHA-256 digest, comparing against manifest declarations.

## Allowed dependencies

- `alp-core::CoreError`
- `crate::model`, `crate::sha256`
- `dirs`, Standard library filesystem

## Threading / real-time notes

File operations and verification perform blocking I/O and run on background worker threads or CLI routines.

## Error handling

Returns `CoreError::Validation` when models are missing or corrupt, and converts `std::io::Error` into `CoreError::Io`.

## Testing

Unit tests in `manager.rs` verify built-in model verification and a full round-trip of file installation, checksum verification, and removal.

## Known limits

- Network download transport is driven by outer tasks or CLI helpers; `ModelManager` manages local file persistence, integrity, and licenses.
