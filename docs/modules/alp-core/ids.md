# `alp-core::ids`

## Purpose

UUID-backed newtypes prevent project, source, track and clip identifiers from
being mixed up at call sites. They serialise as transparent strings and remain
stable across saves.

## Public API

- `ProjectId`, `SourceId`, `TrackId`, `ClipId`
- `new()`, `from_u128(value)`, `as_uuid()`, `Default`, `Display`, `Debug`
- Serde `Serialize`/`Deserialize` as UUID strings

## Data flow

New ids are random v4 UUIDs. Fixtures and tests use `from_u128` for
determinism. JSON stores the hyphenated UUID text; deserialisation rejects
malformed text.

## Allowed dependencies

`uuid` and `serde`. No other ALP crate.

## Threading / real-time notes

`Copy` value types with no interior mutability. Safe to move between threads,
but generating a random UUID can allocate/OS-call and is not for the audio
thread.

## Error handling

Deserialisation errors come from serde/uuid. There are no fallible constructors
besides parsing.

## Testing

Tests round-trip a deterministic ID through JSON and check that freshly
generated IDs differ. Type separation is enforced by the newtype macro rather
than a runtime test.

## Known limits

- Ids are random, not content-addressed.
- No nil-id sentinel is exposed.
