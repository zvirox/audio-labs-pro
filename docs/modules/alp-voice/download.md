# `alp-voice` / `download`

## Purpose

Resumable HTTPS downloads for model manifests. Streams each manifest file into a `<target>.part` companion, verifies manifest size and SHA-256 digest, and renames the part onto the final target only when both hold. Enforces the model-manager gates (unknown model, built-in, offline mode, license acceptance) before any bytes move, and emits throttled progress callbacks for CLI and UI consumers.

## Public API

- `ModelDownloader`: `new` (HTTPS via `HttpSource`), `with_source(Box<dyn ByteSource + Send + Sync>)` (tests, mirrors), `download(&ModelManager, &ModelId, &mut dyn FnMut(DownloadProgress)) -> Result<DownloadReport, DownloadError>`
- `HttpSource`: rustls-backed blocking agent; `USER_AGENT` constant set on every request
- `ByteSource`: transport trait `length(url)` and `read_from(url, start)`
- `DownloadProgress`: `file_index`, `file_count`, `file_name`, `file_done`, `file_total`, `overall_done`, `overall_total` (serde-serializable for future socket/UI transport)
- `DownloadReport`: `model_id`, `files_fetched`, `files_skipped`, `bytes_downloaded`
- `DownloadError`: the full failure taxonomy (see Error handling)

## Data flow

1. Gates run first, in order: manifest lookup, built-in check, offline check, license check. Nothing is created on disk before all four pass.
2. Per file: a present target whose length and SHA-256 match the manifest is reused (reruns are no-ops); corrupt or wrongly sized targets are deleted and refetched.
3. An existing `.part` is treated as a byte prefix: it is folded into the same incremental `Sha256` state the incoming bytes flow into; an oversized part restarts from zero; a complete matching part finalizes by rename without network.
4. `HttpSource::length` probes with `Range: bytes=0-0` (Hugging Face answers 206 plus `Content-Range` total); the total must equal the manifest size before transfer.
5. Free disk space is checked via `fs4::available_space`; if the filesystem refuses to answer, the check is skipped.
6. The stream is read in 64 KiB chunks into the `.part` and the hasher; EOF short of the manifest size or a mid-stream read error keeps the `.part` for the next resume.
7. A full-length stream whose digest mismatches the manifest removes the `.part`; a matching one is flushed, synced, and renamed atomically.
8. Progress fires once per file completion and at most every `max(manifest_total / 100, 4 MiB)` during streaming.

## Allowed dependencies

- `crate::manager::ModelManager`, `crate::model` (manifest types), `crate::sha256`
- `ureq` (default features off; `rustls`, `gzip`), `fs4` (`sync`)
- `serde` (derive), `thiserror`
- Standard library (`fs`, `io`, `path`, `sync`, `time`)

## Threading / real-time notes

All work is blocking I/O (TLS reads, file writes, hashing) and must run on CLI execution paths or dedicated worker threads, never on the real-time audio callback. The downloader itself is single-threaded and sequential (one file at a time, one connection per stream). The `Box<dyn ByteSource + Send + Sync>` bound lets a downloader be constructed on one thread and used from another; progress callbacks run inline on the downloading thread.

## Error handling

`DownloadError` distinguishes every recoverable shape: `UnknownModel` / `NothingToDownload` / `Offline` / `LicenseNotAccepted` (gates, raised before filesystem writes), `Network` (transport, wraps `ureq::Error`), `NoRangeSupport` (server declined ranges or no usable length), `UpstreamSizeMismatch` (length disagreement, before bytes move), `Interrupted` (mid-stream read error; part kept), `ShortStream` (clean early EOF; part kept), `ChecksumMismatch` (digest mismatch; part removed, target absent), `InsufficientSpace` (early refusal), `InvalidFilename` (manifest path escaping the model directory), and `Io` (filesystem failures).

## Testing

14 unit tests in `download.rs` drive the downloader through a recording in-memory `ByteSource` (claimed lengths, capped bodies, requested-offset log): fresh installs with nested paths, resume from a part boundary, oversized-part restart, complete-part finalization without network, corrupt and wrong-size target refetch, checksum mismatch cleanup, short stream kept then resumed after the source recovers, upstream length rejection before any read, progress totals and throttling, and all four gates plus path-traversal rejection. One `manager.rs` test pins license-marker persistence across manager instances. Live end-to-end behavior against Hugging Face (download, zero-network rerun, CDN range resume) is recorded in ADR 0022 and is intentionally not part of the test suite.

## Known limits

- Only a connect timeout exists (15 s); a connection that stalls mid-stream without erroring can hang the download indefinitely.
- When a server compresses a response, `Content-Length` refers to compressed bytes; the length check could then reject a legitimate file. Hugging Face does not compress binary model payloads, so this is theoretical today.
- One sequential stream per file; no parallel segment download, queueing, cancellation, or bandwidth throttling.
- Part durability assumes process crashes, not power loss: the completed chunk buffer is flushed and `sync_all`-ed before rename, but mid-stream bytes rely on OS page cache semantics.
- Progress is throttled, so very small files may only ever report their completion event.
