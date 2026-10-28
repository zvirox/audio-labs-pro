# `alp-api` / `error`

## Purpose

Defines strongly typed error representations for automation actions, permission violations, and sandbox rejections.

## Public API

- `ApiError`: primary error enumeration covering NotFound, Validation, Conflict, PermissionDenied, SandboxEscape, and Core/Media/IO wrappers.

## Data flow

1. Domain operations return `Result<T, ApiError>`.
2. Upper layers (e.g., `alp-mcp`) format `ApiError` into structured MCP tool responses with actionable recovery text.

## Allowed dependencies

- `alp_core::CoreError`
- `alp_media::MediaError`
- `thiserror::Error`

## Threading / real-time notes

Pure error definitions with zero runtime overhead or background thread interaction.

## Error handling

Provides typed variants and conversions from underlying core, media, and I/O errors.

## Testing

Verified across unit tests across `alp-api` and `alp-mcp`.

## Known limits

- None.
