# # `alp-core::commands::tests`

## Purpose

Unit tests for the Phase 1, Phase 3 and Phase 4 command set.

## Public API

Unit tests for the Phase 1, Phase 3 and Phase 4 command set.

## Data flow

Tests build a modest project fixture, run commands through `CommandStack`, and compare serialised project state after undo and redo.

## Allowed dependencies

`alp-core` only.

## Threading / real-time notes

No audio-thread use.

## Error handling

Failures assert exact states.

## Testing

Coverage is growing; all 23 previous and two new command types now have basic tests.

## Known limits

More Phase 4 combinations remain to be exercised.
