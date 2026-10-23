# # `alp-core::commands::source`

## Purpose

Undoable commands for adding and removing imported sources.

## Public API

Undoable commands for adding and removing imported sources.

## Data flow

Indices are validated against the source list and the removed tuple records the restoration position.

## Allowed dependencies

`alp-core` only.

## Threading / real-time notes

No audio-thread use.

## Error handling

Source removal currently relies on the caller ensuring no clips reference it.

## Testing

Covered indirectly by higher-level tests; direct source command tests are still needed.

## Known limits

No source rename or metadata command yet.
