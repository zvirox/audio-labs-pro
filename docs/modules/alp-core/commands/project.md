# # `alp-core::commands::project`

## Purpose

Project-level commands, currently `RenameProject`.

## Public API

Project-level commands, currently `RenameProject`.

## Data flow

Stores the previous project name and restores it on revert.

## Allowed dependencies

`alp-core` only.

## Threading / real-time notes

No audio-thread use.

## Error handling

Empty names are rejected before mutation.

## Testing

Covered indirectly by higher-level tests and the property test.

## Known limits

More project settings commands will arrive with future phases.
