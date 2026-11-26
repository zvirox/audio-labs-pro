# `alp-media` / `m4b`

## Purpose

Native M4B audiobook export. Encodes PCM to AAC-LC with a pure-Rust encoder, writes an ISO-BMFF container, and injects chapter (`chpl`), cover art (`covr`), and iTunes-style metadata (`ilst`) boxes.

## Public API

- `CoverFormat::{Jpeg, Png}`.
- `M4bCover { format, data }`.
- `M4bMetadata { title, narrator, series, series_number, comment }`.
- `M4bChapter { title, start_secs, duration_secs }`.
- `M4bExportReport { path, frames, duration_secs, bitrate, chapter_count, has_cover }`.
- `export_m4b(path, &PcmBuffer, &ExportSettings, &[M4bChapter], Option<&M4bCover>, &M4bMetadata)`.

## Data flow

1. Honour `ExportSettings::sample_rate` when AAC supports it; otherwise resample to 48 kHz.
2. Convert samples to i16 and encode 1024-sample hops with `oxideav_aac::StreamEncoder`.
3. Strip ADTS headers and write AAC payloads as MP4 samples through `mp4::Mp4Writer`.
4. Finalise the container, inject a `udta` box containing `chpl`, `covr`, and `meta/ilst`, then write atomically.

## Allowed dependencies

`alp-core` (atomic writes) plus the MIT-licensed external crates `oxideav-aac` (AAC-LC encoding), `mp4` (ISO-BMFF writing), and `bytes` (sample buffers). No new internal crate dependency.

## Threading / real-time notes

Fully offline operation. Encoding and box injection are CPU-bound and should be run from a worker/job thread; no audio callback or shared state is touched.

## Error handling

Unsupported channel counts, invalid AAC rates, encoder failures, and malformed ADTS frames return `MediaError::Encode` with context. Output is written via `write_atomic`, so a failed export never leaves a partial file at the destination.

## Testing

An integration-style unit test exports a short mono tone with two chapters, fake JPEG cover bytes, and full metadata; it asserts the file exists, reports frames/chapters/cover, and contains `chpl`, `covr`, and `ilst` markers.

## Known limits

- Only mono and stereo AAC-LC are supported.
- Cover art is embedded as JPEG or PNG only.
- Chapter times are stored in a 1 kHz timescale (`start`/`duration` in milliseconds).
- The `chpl`/`covr`/`ilst` boxes are injected by a byte-level post-pass; unusual MP4 writers with 64-bit `moov` sizes are not handled (the bundled `mp4` writer uses 32-bit headers).
