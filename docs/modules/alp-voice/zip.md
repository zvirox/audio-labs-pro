# `alp-voice` / `zip`

## Purpose

Provides a lightweight, self-contained, zero-dependency ZIP archive writer and reader using standard PKWARE method 0 (STORED) and IEEE 802.3 CRC-32 for `.alpvoice` profile bundle packaging and extraction.

## Public API

- `crc32(data: &[u8]) -> u32`
- `ZipWriter<W: Write + Seek>`: `new`, `add_file`, `finish`
- `ZipReader`: `new`, `get`, `filenames`

## Data flow

1. `ZipWriter` writes local file headers, file contents, central directory entries, and end of central directory (EOCD) records.
2. `ZipReader` scans the stream from the EOCD record, parses the central directory, and exposes random-access file retrieval with CRC verification.

## Allowed dependencies

- `alp-core::CoreError`
- Standard library (`std::io::{Cursor, Read, Seek, Write}`, `std::collections::HashMap`)

## Threading / real-time notes

Archive serialization and extraction run synchronously on background workers or CLI invocation threads. Never called in audio callbacks.

## Error handling

Returns `CoreError::Io` on filesystem or stream errors, and `CoreError::Validation` if archive headers are corrupt or CRC mismatches occur.

## Testing

Unit tests in `zip.rs` verify standard CRC-32 vector calculation against standard vectors and full write/read round-trip of multiple files.

## Known limits

- Method 0 (uncompressed) only; ideal for pre-compressed audio and binary embeddings.
