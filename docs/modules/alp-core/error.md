# `alp-core::error`

## Purpose

One error type for project validation, identifiers, persistence, schema and
logging failures. Lookup/command errors live in `CommandError`.

## Public API

- `CoreError` variants: `InvalidSampleRate`, `InvalidRange`, `Validation`,
  `UnsupportedSchemaVersion`, `MissingSchemaVersion`, `InvalidProjectFile`,
  `Io`, `Json`, `MissingDirectory`, `Logging`
- `CoreError::validation(message)`

## Data flow

Constructors and loaders return `CoreError`; callers either display it or wrap
it in `CommandError`. I/O and serde errors convert through `#[from]` so `?`
works naturally.

## Allowed dependencies

`std`, `serde_json`, `thiserror`. No other ALP crate.

## Threading / real-time notes

Error values are plain data and may cross threads. Constructing or formatting
them may allocate, so not for the audio callback.

## Error handling

Messages are user-facing and include the offending id or path. Internal
invariant failures are represented, not panicked.

## Testing

`error.rs` has no dedicated test module. Variant assertions live in the
project, schema, path and media tests that produce each error.

## Known limits

- Errors are stringly for context; there is no structured error code yet.
- No backtrace capture.
