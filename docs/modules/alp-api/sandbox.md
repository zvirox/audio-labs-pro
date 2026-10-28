# `alp-api` / `sandbox`

## Purpose

Restricts filesystem access to authorized workspace boundaries, preventing directory traversal (`..`) and unauthorized file access.

## Public API

- `WorkspaceSandbox::new(roots) -> Result<Self, ApiError>`
- `WorkspaceSandbox::add_root(&mut self, path) -> Result<(), ApiError>`
- `WorkspaceSandbox::validate_path(&self, candidate) -> Result<PathBuf, ApiError>`

## Data flow

1. Callers submit candidate file paths for reading, importing, or exporting.
2. `validate_path` checks for `..` components, resolves relative paths against the root, and verifies the resolved path starts with an allowed root.

## Allowed dependencies

- `std::path::{Path, PathBuf}`
- `crate::error::ApiError`

## Threading / real-time notes

Path canonicalization and filesystem stat calls occur on worker threads.

## Error handling

Returns `ApiError::SandboxEscape` if a path traverses outside permitted roots.

## Testing

Covered by unit tests attempting parent traversal and absolute unauthorized path access.

## Known limits

- Requires existing roots to be canonicalizable.
