# `alp-core::schema`

## Purpose

Defines the versioned JSON envelope around `Project` and the forward-migration
hook. The version sits beside the payload so future migrations can transform
raw JSON before deserialisation.

## Public API

- `CURRENT_SCHEMA_VERSION` (1)
- `to_json(project)`, `from_json(text)`
- `schema_version(value)`, `migrate(value)`

## Data flow

`to_json` validates and emits `{ "schema_version": 1, "project": { ... } }` with
pretty formatting. `from_json` parses a `Value`, checks the version, runs
`migrate`, extracts `project` and deserialises. Newer files are rejected.

## Allowed dependencies

`serde`, `serde_json`, `CoreError`, `Project`.

## Threading / real-time notes

Allocations and string parsing; do not call from the audio callback. The
resulting `Project` is plain owned data.

## Error handling

Missing/non-integer versions produce `MissingSchemaVersion`; newer versions
produce `UnsupportedSchemaVersion`; missing payloads produce
`InvalidProjectFile`; JSON syntax errors come from `serde_json`.

## Testing

Tests round-trip a project, reject a future schema version and reject a
document without `schema_version`. Version 0 and envelope-shape cases are not
dedicated tests yet.

## Known limits

- Only version 1 exists; `migrate` is a scaffold with no historical arms.
- No schema fingerprint or checksum.
