# ADR 0022: Resumable Model Downloads in `alp-voice` via Blocking `ureq` + `fs4`

- **Status:** Accepted
- **Date:** 2026-10-29
- **Deciders:** Audio Labs Pro Engineering
- **Consulted:** Hugging Face Hub access docs, `ureq` 3.x migration guide, `fs4` crate docs

## Context and Problem Statement

Phase 6 wired the Kokoro-82M neural tier into `alp-infer`, but model files still had to be fetched by hand: manifests (sizes, SHA-256 digests, source URLs) existed in `alp-voice`, and `alp-cli model verify` could prove an installation sound, yet nothing in the product could *produce* an installation. The 325 MB Kokoro payload and the ~23 MB kitten-nano model exceed what anyone should download through `curl` copy-paste instructions, and a failed 325 MB transfer with no resume is unacceptable UX.

The downloader must satisfy the project's standing constraints: license purity (no GPL, no OpenSSL-native linkage surprises), a synchronous architecture (no async runtime in `alp-voice`), gate enforcement before bytes move (license acceptance, offline mode, built-in models), and tamper evidence (a file is only "installed" once it hashes to its manifest digest).

## Decision Drivers

1. **Resumability:** interrupted transfers must continue from the last byte boundary, not restart a 325 MB fetch.
2. **Integrity:** SHA-256 verification is mandatory before a file becomes final; a mismatch must never leave a half-trusted target on disk.
3. **Licensing purity:** pure-Rust TLS (rustls); no OpenSSL, no GPL, no external process dependency.
4. **Synchronous codebase:** `alp-voice` is a sync library used by the CLI and the daemon's worker threads; an async HTTP client would drag tokio into every consumer.
5. **Hugging Face as primary host:** every request needs a `User-Agent` (bare requests are rejected), and byte-range support is confirmed (`Range: bytes=0-0` answers 206 with a `Content-Range` total; redirects land on the CDN, which honors open-ended ranges).
6. **Configurability for tests:** transport must be injectable so tests exercise resume/corruption paths without network.

## Considered Options

- **Option 1: Blocking `ureq` 3.5 with rustls (Chosen)**
  - Small, pure-Rust stack (`ureq` + `rustls`); MIT OR Apache-2.0.
  - Native blocking model matches the CLI and worker-thread call sites.
  - Explicit `User-Agent` and `Range` headers per request; 4xx/5xx surface as `Err`.
- **Option 2: `reqwest` blocking client**
  - Pulls tokio, hyper, and a large tree into every `alp-voice` consumer for a feature (one GET at a time) we barely use; the blocking facade still builds a tokio runtime internally.
  - Heavier audit surface for no functional gain over Option 1.
- **Option 3: Shell out to `curl` or `wget`**
  - Breaks self-contained packaging (external binary required), progress parsing is fragile, and error taxonomy would be stringly typed.
- **Option 4: Hand-rolled HTTPS on `std::net::TcpStream`**
  - Requires implementing TLS or unsafe FFI bindings; correctness risk is unacceptable for a program that verifies model weights.

Disk-space early refusal uses **`fs4` 1.1** (`sync` feature) over raw `libc::statvfs` FFI: same answer, safe Rust, MIT OR Apache-2.0.

## Decision

`alp-voice` gains a `download` module around three abstractions:

- `ByteSource` trait (`length`, `read_from`): transport injection seam; `HttpSource` implements it with `ureq` (rustls, gzip, 15 s connect timeout) and sets `USER_AGENT` on every request.
- `ModelDownloader::download(manager, id, progress_cb)`: gates in fixed order (unknown model -> built-in -> offline -> license), then per file: reuse a present target whose SHA-256 matches; otherwise stream into `<target>.part`, folding any pre-existing part prefix and the incoming bytes into one incremental `Sha256`. A completed stream must land exactly on the manifest size and digest before an atomic rename makes it final.
- Progress reporting throttled to at most ~1 percent of the model or 4 MiB steps (whichever is larger), with a guaranteed tick per file completion, so CLI and future UI surfaces get clean updates without per-chunk spam.

Failure invariants: clean early EOF or a mid-stream transport error **keeps** the `.part` (it is a valid prefix, perfect for resume); a digest mismatch **removes** the `.part`; an upstream length disagreement aborts before a single byte transfers. Alongside this, `ModelManager::accept_license` now persists a `licenses/<id>.accepted` marker under the storage root and `with_config` restores them, so the license gate stays open across processes (`alp-cli model accept-license` finally outlives its own invocation).

## Consequences

- Two new dependencies in `alp-voice`: `ureq` 3.5 (rustls + gzip, default features off) and `fs4` 1.1 (`sync`). Both pure Rust, both MIT OR Apache-2.0.
- Live end-to-end evidence on Hugging Face: kitten-nano (12 files) downloaded and verified; a rerun skips all 12 files with zero network; deleting the weights file and leaving a 1,000,000-byte `.part` resumed over a real CDN 206, fetching exactly the 21.7 MB remainder.
- Verification is unchanged downstream: `alp-cli model verify` (`ModelManager::verify`) reads the same manifests.
- Single sequential stream per file; parallel segment downloading (aria2-style) is deliberately out of scope for v1.
- Connect timeout only (15 s); a connection that stalls mid-stream without erroring is not yet timed out (see known limits of `docs/modules/alp-voice/download.md`).
- `alp-infer`'s missing-weights error now points at the real command (`alp-cli model download <id>`), removing the earlier workaround phrasing.
