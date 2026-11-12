# `alp-infer` / `gguf`

## Purpose

A small, dependency-free GGUF header reader used to validate heavy-model
installations before their runtime is loaded.

The heavy clone tier ships as GGUF files. Before the runtime is loaded, the
engine checks that the file really is a GGUF container and that it carries the
architecture the engine expects — a wrong or truncated file is then reported
as such instead of surfacing later as a confusing tensor error.

Only the header is parsed: the magic, the version, the tensor and metadata
counts, and the metadata key/value pairs. Tensor payloads are never read, so
validation costs a few kilobytes of I/O regardless of model size.

## Public API

- `read_header(path: &Path) -> Result<GgufHeader, GgufError>`: parse the
  header of a GGUF file.
- `GgufHeader`: `version`, `tensor_count`, `metadata_count`, `metadata`.
  - `get(key) -> Option<&MetadataValue>`: look up a metadata entry.
  - `architecture() -> Option<&str>`: the `general.architecture` value.
  - `u64(key) -> Option<u64>`: a metadata value as an unsigned integer.
- `MetadataValue`: `U64`, `I64`, `F32`, `Bool`, `String`, `Array(usize)`.
  Large arrays are summarized by their length rather than materialized — a
  vocabulary can hold 150 000 entries.
- `GgufError`: `Io`, `NotGguf`, `UnsupportedVersion`, `UnknownValueType`,
  `Truncated`.

## Data flow

1. The file is opened and wrapped in a `BufReader`.
2. The four magic bytes are read and compared with `GGUF`; a mismatch is
   `NotGguf`.
3. The version is read and checked against the highest version this reader
   understands (3); a newer file is `UnsupportedVersion`.
4. The tensor count and metadata count are read.
5. Each metadata entry is read as a key string followed by a typed value.
   Arrays are walked with their element type and skipped without being
   materialized.
6. The parsed entries are returned as `GgufHeader`.

## Allowed dependencies

- `thiserror`: the error enum
- Standard library only (`std::fs`, `std::io`)

## Threading / real-time notes

Blocking file I/O. It runs during model load (a job worker in the UI, a
one-shot CLI command), never on the real-time audio callback. The reader
takes no shared state and is safe to call concurrently on distinct files.

## Error handling

- A missing or unreadable file is `GgufError::Io` with the path and the
  underlying error.
- A file that is not GGUF is `NotGguf`.
- A header that ends early is `Truncated`.
- A string length claiming an implausible size (> 64 MiB) is treated as
  `Truncated` rather than attempting the allocation.
- A value type this reader does not know is `UnknownValueType`.

## Testing

Unit tests in `gguf.rs` verify that:

- the architecture string is read from a minimal GGUF v3 header;
- a file that is not GGUF is rejected with `NotGguf`;
- a truncated header is rejected with `Truncated`;
- a missing file reports an I/O error.

## Known limits

- Only the header is read; the tensor directory and payloads are not parsed,
  so the reader cannot detect a file whose header is valid but whose tensor
  data is corrupt.
- Version 2 files are accepted but not distinguished from version 3.
- Nested arrays and the `Array` value type are summarized by length only.
