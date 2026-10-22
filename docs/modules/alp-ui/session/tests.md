# `alp-ui` / `session/tests`

## Purpose

Unit tests for the live editing session's commands, undo behavior, background
import, punch-in and autosave recovery.

## Public API

No public API. The module is compiled only under `cfg(test)`.

## Data flow

Tests build `EditSession::demo()` for pure in-memory cases, or
`EditSession::open_or_demo()` against a `tempfile` directory for persistence.
They call session methods exactly as the reducer does and assert on project
state, clip counts, undo availability and validated recovery.

## Allowed dependencies

The parent `session` module, `alp-media` test-tone generation and `tempfile`.

## Threading / real-time notes

The background-import test polls a real worker with a bounded sleep loop. It
does not use the audio callback or a device.

## Error handling

Tests assert that command failures leave a valid project after undo and that a
corrupt or missing recovery path falls back without panicking.

## Testing

This module is the test suite itself. Run with `cargo test -p alp-ui`.

## Known limits

- Hardware capture and actual device recording remain manual acceptance.
- The autosave test uses an explicit flush rather than waiting for the 750 ms
  debounce.
