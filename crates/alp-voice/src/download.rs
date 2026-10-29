//! Resumable HTTPS downloads for model manifests with SHA-256 verification.
//!
//! Files stream to a `<target>.part` companion so an interrupted download can
//! resume from the byte boundary already on disk. Only after a file reaches its
//! manifest size and its SHA-256 digest matches the manifest is the `.part`
//! renamed onto the final target. A throttled progress callback keeps the CLI
//! (and, later, the Voice Lab UI) informed without filesystem polling.
//!
//! Gating (checked in this order before any bytes move): unknown model, built-in
//! model, offline mode, unaccepted license.

use crate::manager::ModelManager;
use crate::model::{ModelFile, ModelId};
use crate::sha256::Sha256;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufReader, Read, Write};
use std::path::{Component, Path, PathBuf};

/// Lower bound of the progress reporting step: at most ~1 percent of the model,
/// never fewer than 4 MiB between callbacks during streaming.
const PROGRESS_MIN_STEP: u64 = 4 * 1024 * 1024;

/// Streaming buffer size for hashing and disk writes.
const READ_CHUNK: usize = 64 * 1024;

/// `User-Agent` sent with every upstream request.
///
/// Hugging Face rejects unauthenticated-looking requests that carry no
/// `User-Agent` at all, so every [`HttpSource`] request sets this header.
pub const USER_AGENT: &str = concat!("audio-labs-pro/", env!("CARGO_PKG_VERSION"));

/// Errors produced while downloading a model.
#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    /// No manifest for the requested identifier.
    #[error("unknown model: {0}")]
    UnknownModel(String),
    /// The model ships inside the application binary.
    #[error("{0} is built into the application; there is nothing to download")]
    NothingToDownload(String),
    /// Offline mode forbids network access.
    #[error("offline mode is enabled; model downloads are disabled")]
    Offline,
    /// The model license has not been accepted in this storage root.
    #[error(
        "the license for {id} must be accepted before downloading; \
             run `alp-cli model accept-license {id}`"
    )]
    LicenseNotAccepted {
        /// Model identifier whose license is still pending.
        id: String,
    },
    /// Transport failure (DNS, TLS, connection reset, HTTP 4xx/5xx).
    #[error("network error fetching {url}: {source}")]
    Network {
        /// URL that failed.
        url: String,
        /// Underlying agent error.
        source: ureq::Error,
    },
    /// The server does not advertise a usable length / refuses byte ranges.
    #[error("server did not offer resumable byte ranges for {url}")]
    NoRangeSupport {
        /// URL that must be fetched as a whole.
        url: String,
    },
    /// `Content-Length` / `Content-Range` disagrees with the manifest.
    #[error("upstream size for {url} is {actual} bytes but the manifest expects {expected}")]
    UpstreamSizeMismatch {
        /// URL whose length disagrees.
        url: String,
        /// Byte size declared by the manifest.
        expected: u64,
        /// Byte size reported by the server.
        actual: u64,
    },
    /// Connection broke before the manifest size was reached.
    #[error(
        "download for {url} was interrupted after {received} bytes \
             (partial file kept for resume)"
    )]
    Interrupted {
        /// URL whose stream broke.
        url: String,
        /// Bytes delivered before the failure.
        received: u64,
        /// Underlying read error.
        source: std::io::Error,
    },
    /// The server closed the stream cleanly but early.
    #[error(
        "download for {url} ended early: {received} of {expected} bytes \
             (partial file kept for resume)"
    )]
    ShortStream {
        /// URL whose stream ended early.
        url: String,
        /// Byte size declared by the manifest.
        expected: u64,
        /// Bytes actually delivered.
        received: u64,
    },
    /// The assembled file does not hash to the manifest digest.
    #[error("checksum mismatch for {filename}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        /// File (manifest-relative) whose digest disagreed.
        filename: String,
        /// Digest declared by the manifest.
        expected: String,
        /// Digest computed from the downloaded bytes.
        actual: String,
    },
    /// Free disk space cannot cover the remaining bytes.
    #[error(
        "insufficient disk space for {filename}: need at least {needed} bytes, \
             only {available} available"
    )]
    InsufficientSpace {
        /// File that could not fit.
        filename: String,
        /// Bytes still required.
        needed: u64,
        /// Bytes free at the destination.
        available: u64,
    },
    /// A manifest filename escapes the model directory.
    #[error("invalid model file path in manifest: {0}")]
    InvalidFilename(String),
    /// Filesystem failure.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// One progress report issued by the downloader while a model downloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadProgress {
    /// Zero-based index of the file within the manifest.
    pub file_index: usize,
    /// Total file count in the manifest.
    pub file_count: usize,
    /// Manifest-relative filename currently being fetched.
    pub file_name: String,
    /// Bytes completed for the current file.
    pub file_done: u64,
    /// Manifest size of the current file.
    pub file_total: u64,
    /// Bytes completed across the whole model (fetched + reused + skipped).
    pub overall_done: u64,
    /// Total manifest size of the model.
    pub overall_total: u64,
}

/// Outcome summary of [`ModelDownloader::download`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadReport {
    /// Model identifier that was downloaded.
    pub model_id: String,
    /// Files fetched over the network during this run.
    pub files_fetched: Vec<String>,
    /// Files already present and matching (or resumed to completion without
    /// being fully refetched).
    pub files_skipped: Vec<String>,
    /// Network bytes transferred during this run (resumed prefixes excluded).
    pub bytes_downloaded: u64,
}

/// A byte source the downloader can probe and stream from.
///
/// Implementations answer two questions for a manifest URL: how long the
/// resource is, and where a reader starting at a given offset is obtained. The
/// in-tree implementation is [`HttpSource`]; tests drive the downloader with
/// in-memory sources instead of the network.
pub trait ByteSource {
    /// Report the total byte length of the resource at `url`.
    fn length(&self, url: &str) -> Result<u64, DownloadError>;

    /// Open a reader over the bytes of `url` starting at byte offset `start`.
    fn read_from(&self, url: &str, start: u64) -> Result<Box<dyn Read + '_>, DownloadError>;
}

/// [`ByteSource`] over HTTPS using a blocking rustls agent.
///
/// Deliberately chosen over `reqwest` (tokio + OpenSSL baggage) and shelling out
/// to curl: the downloader is synchronous, must not drag an async runtime into
/// `alp-voice`, and keeps licensing pure (rustls, MIT OR Apache-2.0).
pub struct HttpSource {
    agent: ureq::Agent,
}

impl Default for HttpSource {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpSource {
    /// Create a source with sensible connect timeouts and no proxy config.
    #[must_use]
    pub fn new() -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_connect(Some(std::time::Duration::from_secs(15)))
            .build();
        Self {
            agent: config.new_agent(),
        }
    }

    /// Issue a GET with the mandatory `User-Agent` and an optional `Range`.
    fn call(
        &self,
        url: &str,
        range: Option<&str>,
    ) -> Result<ureq::http::Response<ureq::Body>, DownloadError> {
        let mut request = self.agent.get(url).header("User-Agent", USER_AGENT);
        if let Some(range) = range {
            request = request.header("Range", range);
        }
        request.call().map_err(|source| DownloadError::Network {
            url: url.to_string(),
            source,
        })
    }
}

impl ByteSource for HttpSource {
    fn length(&self, url: &str) -> Result<u64, DownloadError> {
        // A 1-byte range probe doubles as an existence check; Hugging Face
        // answers with 206 Plus `Content-Range: bytes 0-0/<total>`.
        let response = self.call(url, Some("bytes=0-0"))?;
        match response.status().as_u16() {
            206 => {
                let raw = response
                    .headers()
                    .get("Content-Range")
                    .and_then(|value| value.to_str().ok());
                match raw.and_then(parse_content_range_total) {
                    Some(total) => Ok(total),
                    None => Err(DownloadError::NoRangeSupport { url: url.into() }),
                }
            }
            // Some servers ignore Range and return the whole resource; the
            // Content-Length then already is the full size.
            _ => {
                let raw = response
                    .headers()
                    .get("Content-Length")
                    .and_then(|value| value.to_str().ok());
                raw.and_then(|value| value.trim().parse().ok())
                    .ok_or_else(|| DownloadError::NoRangeSupport { url: url.into() })
            }
        }
    }

    fn read_from(&self, url: &str, start: u64) -> Result<Box<dyn Read + '_>, DownloadError> {
        // An open-ended range lets CDNs answer from their middle-of-file caches.
        let range = (start > 0).then(|| format!("bytes={start}-"));
        let response = self.call(url, range.as_deref())?;
        match (response.status().as_u16(), start) {
            (206, _) | (200, 0) => {}
            // A plain 200 while resuming would deliver the file from byte 0 and
            // silently corrupt the appended offset; refuse it.
            _ => return Err(DownloadError::NoRangeSupport { url: url.into() }),
        }
        Ok(Box::new(response.into_body().into_reader()))
    }
}

/// Parse the total length out of a `Content-Range` header value.
fn parse_content_range_total(value: &str) -> Option<u64> {
    value.rsplit('/').next()?.trim().parse().ok()
}

/// Downloads model files named by a [`ModelManifest`] into a
/// [`ModelManager`](crate::manager::ModelManager) storage root.
pub struct ModelDownloader {
    source: Box<dyn ByteSource + Send + Sync>,
}

impl Default for ModelDownloader {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelDownloader {
    /// A downloader backed by plain HTTPS.
    #[must_use]
    pub fn new() -> Self {
        Self {
            source: Box::new(HttpSource::new()),
        }
    }

    /// A downloader backed by a custom byte source (tests, mirrors, bundles).
    #[must_use]
    pub fn with_source(source: Box<dyn ByteSource + Send + Sync>) -> Self {
        Self { source }
    }

    /// Download every file of `id`, gating hard before any bytes move.
    ///
    /// Gate order: unknown model, built-in model, offline mode, license. Each
    /// file is written to `<target>.part` and renamed onto the target only once
    /// its manifest size and SHA-256 digest are confirmed, so every failure path
    /// leaves either a resumable `.part` or no half-written file at all.
    pub fn download(
        &self,
        manager: &ModelManager,
        id: &ModelId,
        on_progress: &mut dyn FnMut(DownloadProgress),
    ) -> Result<DownloadReport, DownloadError> {
        let manifest = manager
            .get_manifest(id)
            .ok_or_else(|| DownloadError::UnknownModel(id.to_string()))?;
        if manifest.is_builtin {
            return Err(DownloadError::NothingToDownload(id.to_string()));
        }
        if manager.is_offline() {
            return Err(DownloadError::Offline);
        }
        if !manifest.license.accepted {
            return Err(DownloadError::LicenseNotAccepted { id: id.to_string() });
        }

        let model_dir = manager.model_dir(id);
        fs::create_dir_all(&model_dir)?;

        let total_bytes = manifest.total_size_bytes();
        let file_count = manifest.files.len();
        let mut report = DownloadReport {
            model_id: id.to_string(),
            files_fetched: Vec::new(),
            files_skipped: Vec::new(),
            bytes_downloaded: 0,
        };
        let mut bytes_done_before: u64 = 0;

        for (index, file) in manifest.files.iter().enumerate() {
            let outcome = self.download_file(
                &model_dir,
                file,
                index,
                file_count,
                bytes_done_before,
                total_bytes,
                on_progress,
            )?;
            if outcome.fetched {
                report.files_fetched.push(outcome.filename);
                report.bytes_downloaded += outcome.bytes_from_network;
            } else {
                report.files_skipped.push(outcome.filename);
            }
            bytes_done_before += file.size_bytes;
        }
        Ok(report)
    }

    /// Fetch (or reuse) a single manifest file inside `model_dir`.
    #[allow(clippy::too_many_arguments)]
    fn download_file(
        &self,
        model_dir: &Path,
        file: &ModelFile,
        index: usize,
        count: usize,
        bytes_done_before: u64,
        total_bytes: u64,
        on_progress: &mut dyn FnMut(DownloadProgress),
    ) -> Result<FileOutcome, DownloadError> {
        validate_filename(&file.filename)?;
        let target = model_dir.join(&file.filename);
        let part = part_path(model_dir, &file.filename);
        // At most ~1 percent steps, never below 4 MiB, so progress lines stay
        // readable even over fast links.
        let step = std::cmp::max(total_bytes / 100, PROGRESS_MIN_STEP);

        // A present target is only trusted when it hashes to the manifest
        // digest: reruns of a completed download become a no-op, and silently
        // corrupted or truncated targets are refetched instead of believed.
        if target.is_file() && fs::metadata(&target)?.len() == file.size_bytes {
            let hex = hash_file(&target)?;
            if hex.eq_ignore_ascii_case(&file.sha256) {
                emit_progress(
                    on_progress,
                    index,
                    count,
                    &file.filename,
                    file.size_bytes,
                    file.size_bytes,
                    bytes_done_before + file.size_bytes,
                    total_bytes,
                );
                return Ok(FileOutcome {
                    filename: file.filename.clone(),
                    fetched: false,
                    bytes_from_network: 0,
                });
            }
        }
        if target.exists() {
            fs::remove_file(&target)?;
        }

        // Resume bookkeeping: an existing `.part` represents bytes already on
        // disk. They are folded into the same hasher the network bytes flow
        // into, so the final digest always covers the complete file.
        let mut base: u64 = 0;
        let mut hasher = Sha256::new();
        if part.is_file() {
            let part_len = fs::metadata(&part)?.len();
            if part_len > file.size_bytes {
                // Oversized: cannot be a prefix of the real file; start over.
                fs::remove_file(&part)?;
            } else {
                let mut reader = fs::File::open(&part)?;
                copy_into_hasher(&mut reader, &mut hasher, None)?;
                drop(reader);
                base = part_len;
                if base == file.size_bytes {
                    // Complete `.part`, e.g. a crash between download and
                    // rename: verify and finalize without any network traffic.
                    let candidate = std::mem::take(&mut hasher);
                    let hex = finish_hex(candidate);
                    if hex.eq_ignore_ascii_case(&file.sha256) {
                        fs::rename(&part, &target)?;
                        emit_progress(
                            on_progress,
                            index,
                            count,
                            &file.filename,
                            file.size_bytes,
                            file.size_bytes,
                            bytes_done_before + file.size_bytes,
                            total_bytes,
                        );
                        return Ok(FileOutcome {
                            filename: file.filename.clone(),
                            fetched: false,
                            bytes_from_network: 0,
                        });
                    }
                    fs::remove_file(&part)?;
                    base = 0;
                }
            }
        }

        // The upstream resource must be exactly as long as the manifest claims
        // before a single byte is transferred.
        let upstream_len = self.source.length(&file.url)?;
        if upstream_len != file.size_bytes {
            return Err(DownloadError::UpstreamSizeMismatch {
                url: file.url.clone(),
                expected: file.size_bytes,
                actual: upstream_len,
            });
        }

        // Disk space is a best-effort early refusal; filesystems that do not
        // report free space (some overlays / proxies) simply proceed.
        if let Ok(available) = fs4::available_space(model_dir) {
            let needed = file.size_bytes - base;
            if available < needed {
                return Err(DownloadError::InsufficientSpace {
                    filename: file.filename.clone(),
                    needed,
                    available,
                });
            }
        }

        // Nested manifest paths (e.g. `onnx/model.onnx`) need their parents.
        if let Some(parent) = part.parent() {
            fs::create_dir_all(parent)?;
        }

        let reader = self.source.read_from(&file.url, base)?;
        let mut reader = BufReader::with_capacity(READ_CHUNK, reader);
        let mut writer = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&part)?;
        let mut buf = vec![0u8; READ_CHUNK];
        let mut received: u64 = 0;
        let mut last_reported = bytes_done_before + base;

        loop {
            let n = match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                // A transport error mid-stream is a resume point, not a dead
                // end: keep the part and tell the caller how far we got.
                Err(source) => {
                    let _ = writer.flush();
                    return Err(DownloadError::Interrupted {
                        url: file.url.clone(),
                        received: base + received,
                        source,
                    });
                }
            };
            hasher.update(&buf[..n]);
            writer.write_all(&buf[..n])?;
            received += n as u64;

            let overall = bytes_done_before + base + received;
            if overall - last_reported >= step {
                emit_progress(
                    on_progress,
                    index,
                    count,
                    &file.filename,
                    base + received,
                    file.size_bytes,
                    overall,
                    total_bytes,
                );
                last_reported = overall;
            }
        }
        writer.flush()?;
        writer.sync_all()?;

        let file_total = base + received;
        if file_total != file.size_bytes {
            // Clean early EOF: the `.part` is exactly a prefix of the file and
            // stays on disk for the next resume attempt.
            return Err(DownloadError::ShortStream {
                url: file.url.clone(),
                expected: file.size_bytes,
                received: file_total,
            });
        }

        let hex = finish_hex(hasher);
        if !hex.eq_ignore_ascii_case(&file.sha256) {
            // A poisoned part cannot be resumed against; drop it, target stays
            // absent, the next run starts from scratch.
            fs::remove_file(&part)?;
            return Err(DownloadError::ChecksumMismatch {
                filename: file.filename.clone(),
                expected: file.sha256.clone(),
                actual: hex,
            });
        }

        fs::rename(&part, &target)?;
        emit_progress(
            on_progress,
            index,
            count,
            &file.filename,
            file.size_bytes,
            file.size_bytes,
            bytes_done_before + file.size_bytes,
            total_bytes,
        );
        Ok(FileOutcome {
            filename: file.filename.clone(),
            fetched: true,
            bytes_from_network: received,
        })
    }
}

/// Per-file outcome handed back to `download` for report assembly.
struct FileOutcome {
    filename: String,
    fetched: bool,
    bytes_from_network: u64,
}

/// Reject filenames that would escape the model directory.
fn validate_filename(filename: &str) -> Result<(), DownloadError> {
    let path = Path::new(filename);
    if filename.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(DownloadError::InvalidFilename(filename.to_string()));
    }
    Ok(())
}

/// Companion path receiving the streaming bytes of `model_dir/filename`.
fn part_path(model_dir: &Path, filename: &str) -> PathBuf {
    model_dir.join(format!("{filename}.part"))
}

/// Read `reader` to EOF, folding every byte into `hasher` and, when `sink` is
/// supplied, writing them through to a file. Returns the byte count seen.
fn copy_into_hasher(
    reader: &mut dyn Read,
    hasher: &mut Sha256,
    mut sink: Option<&mut fs::File>,
) -> std::io::Result<u64> {
    let mut buf = vec![0u8; READ_CHUNK];
    let mut total = 0u64;
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            return Ok(total);
        }
        hasher.update(&buf[..n]);
        if let Some(sink) = sink.as_deref_mut() {
            sink.write_all(&buf[..n])?;
        }
        total += n as u64;
    }
}

/// Stream a whole file into a fresh hasher and return its lowercase hex digest.
fn hash_file(path: &Path) -> Result<String, DownloadError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    copy_into_hasher(&mut file, &mut hasher, None)?;
    Ok(finish_hex(hasher))
}

/// Finalize `hasher` into a lowercase hex string.
fn finish_hex(hasher: Sha256) -> String {
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Publish one progress tick to the caller.
#[allow(clippy::too_many_arguments)]
fn emit_progress(
    on_progress: &mut dyn FnMut(DownloadProgress),
    file_index: usize,
    file_count: usize,
    file_name: &str,
    file_done: u64,
    file_total: u64,
    overall_done: u64,
    overall_total: u64,
) {
    on_progress(DownloadProgress {
        file_index,
        file_count,
        file_name: file_name.to_string(),
        file_done,
        file_total,
        overall_done,
        overall_total,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::{ModelManager, ModelManagerConfig};
    use crate::model::{ModelKind, ModelLicense, ModelManifest, ModelStatus};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use tempfile::tempdir;

    /// In-memory [`ByteSource`] with a claimed (manifest-facing) length and an
    /// optionally capped served body, recording every requested start offset.
    #[derive(Clone, Default)]
    struct MemSource {
        lengths: HashMap<String, u64>,
        bodies: HashMap<String, Vec<u8>>,
        serve_caps: HashMap<String, u64>,
        reads: Arc<Mutex<HashMap<String, Vec<u64>>>>,
    }

    impl MemSource {
        /// Register a URL whose claimed length equals the body length.
        fn add(&mut self, url: &str, body: &[u8]) {
            let len = body.len() as u64;
            self.lengths.insert(url.into(), len);
            self.bodies.insert(url.into(), body.to_vec());
        }

        /// Cap how many bytes `read_from` will serve from offset 0.
        fn set_serve_len(&mut self, url: &str, cap: u64) {
            self.serve_caps.insert(url.into(), cap);
        }

        fn set_claimed_len(&mut self, url: &str, claimed: u64) {
            self.lengths.insert(url.into(), claimed);
        }

        /// Start offsets requested so far for `url`.
        fn starts(&self, url: &str) -> Vec<u64> {
            self.reads
                .lock()
                .expect("reads lock")
                .get(url)
                .cloned()
                .unwrap_or_default()
        }
    }

    impl ByteSource for MemSource {
        fn length(&self, url: &str) -> Result<u64, DownloadError> {
            Ok(self.lengths.get(url).copied().expect("test URL registered"))
        }

        fn read_from(&self, url: &str, start: u64) -> Result<Box<dyn Read + '_>, DownloadError> {
            self.reads
                .lock()
                .expect("reads lock")
                .entry(url.to_string())
                .or_default()
                .push(start);
            let body = self.bodies.get(url).expect("test URL registered");
            let cap = self.serve_caps.get(url).copied().unwrap_or(u64::MAX);
            let end = (cap.min(body.len() as u64)) as usize;
            let start = start.min(end as u64) as usize;
            Ok(Box::new(std::io::Cursor::new(&body[start..end])))
        }
    }

    /// Assemble a manifest for `test_manager` with a configurable license gate.
    fn manifest_with(files: Vec<ModelFile>, accepted: bool) -> ModelManifest {
        ModelManifest {
            id: ModelId::new("test-model"),
            name: "Test Model".into(),
            description: "test".into(),
            kind: ModelKind::TtsLight,
            author: "test".into(),
            version: "1.0".into(),
            license: ModelLicense {
                name: "MIT".into(),
                spdx: "MIT".into(),
                terms: "MIT terms".into(),
                accepted,
            },
            files,
            is_builtin: false,
            voice_names: vec![],
        }
    }

    /// Build a one-model manager with the supplied files, license pre-accepted.
    fn test_manager(storage: &Path, files: Vec<ModelFile>) -> ModelManager {
        let manifest = manifest_with(files, true);
        let mut manager = ModelManager::with_config(ModelManagerConfig {
            storage_dir: storage.to_path_buf(),
            offline_mode: false,
        });
        manager.register_manifest(manifest);
        manager
    }

    /// Pair a in-memory source with manifest files derived from the bodies.
    fn make_source(spec: &[(&str, &[u8])]) -> (MemSource, Vec<ModelFile>) {
        let mut source = MemSource::default();
        let mut files = Vec::new();
        for (name, body) in spec {
            let url = format!("mem://{name}");
            source.add(&url, body);
            files.push(ModelFile {
                filename: (*name).to_string(),
                size_bytes: body.len() as u64,
                sha256: crate::sha256::digest_hex(body),
                url,
            });
        }
        (source, files)
    }

    #[test]
    fn fresh_download_installs_nested_files() {
        let temp = tempdir().expect("tempdir");
        let (source, files) = make_source(&[
            ("onnx/model.bin", b"weights-bytes"),
            ("tokenizer.json", b"{}"),
        ]);
        let manager = test_manager(temp.path(), files);
        let downloader = ModelDownloader::with_source(Box::new(source.clone()));
        let id = ModelId::new("test-model");

        let report = downloader
            .download(&manager, &id, &mut |_| {})
            .expect("download");

        assert_eq!(report.files_fetched.len(), 2);
        assert!(report.files_skipped.is_empty());
        assert_eq!(report.bytes_downloaded, 15);
        assert_eq!(source.starts("mem://onnx/model.bin"), vec![0]);
        assert!(matches!(
            manager.verify(&id).expect("verify"),
            ModelStatus::Verified { .. }
        ));
        assert!(temp.path().join("test-model/onnx/model.bin").is_file());
        assert!(!temp.path().join("test-model/onnx/model.bin.part").exists());
    }

    #[test]
    fn resume_continues_from_part_offset() {
        let temp = tempdir().expect("tempdir");
        let (source, files) = make_source(&[("weights.bin", b"abcdefghij")]);
        let manager = test_manager(temp.path(), files);
        let id = ModelId::new("test-model");
        fs::create_dir_all(temp.path().join("test-model")).expect("model dir");
        fs::write(temp.path().join("test-model/weights.bin.part"), b"abcd").expect("part");

        let report = ModelDownloader::with_source(Box::new(source.clone()))
            .download(&manager, &id, &mut |_| {})
            .expect("resume");

        assert_eq!(
            source.starts("mem://weights.bin"),
            vec![4],
            "resume must start the stream at the part boundary"
        );
        assert_eq!(report.files_fetched, vec!["weights.bin".to_string()]);
        assert_eq!(report.bytes_downloaded, 6);
        assert_eq!(
            fs::read(temp.path().join("test-model/weights.bin")).expect("target"),
            b"abcdefghij"
        );
        assert!(!temp.path().join("test-model/weights.bin.part").exists());
        assert!(matches!(
            manager.verify(&id).expect("verify"),
            ModelStatus::Verified { .. }
        ));
    }

    #[test]
    fn oversized_part_restarts_from_zero() {
        let temp = tempdir().expect("tempdir");
        let (source, files) = make_source(&[("weights.bin", b"abcdefghij")]);
        let manager = test_manager(temp.path(), files);
        let id = ModelId::new("test-model");
        fs::create_dir_all(temp.path().join("test-model")).expect("model dir");
        fs::write(
            temp.path().join("test-model/weights.bin.part"),
            b"0123456789ABCDEF",
        )
        .expect("oversized part");

        let report = ModelDownloader::with_source(Box::new(source.clone()))
            .download(&manager, &id, &mut |_| {})
            .expect("download");

        assert_eq!(source.starts("mem://weights.bin"), vec![0]);
        assert_eq!(
            fs::read(temp.path().join("test-model/weights.bin")).expect("target"),
            b"abcdefghij"
        );
        assert_eq!(report.bytes_downloaded, 10);
    }

    #[test]
    fn complete_part_finalizes_without_network() {
        let temp = tempdir().expect("tempdir");
        let (source, files) = make_source(&[("weights.bin", b"abcdefghij")]);
        let manager = test_manager(temp.path(), files);
        let id = ModelId::new("test-model");
        fs::create_dir_all(temp.path().join("test-model")).expect("model dir");
        fs::write(
            temp.path().join("test-model/weights.bin.part"),
            b"abcdefghij",
        )
        .expect("complete part");

        let report = ModelDownloader::with_source(Box::new(source.clone()))
            .download(&manager, &id, &mut |_| {})
            .expect("finalize");

        assert!(
            source.starts("mem://weights.bin").is_empty(),
            "a complete, matching part must not touch the network"
        );
        assert!(temp.path().join("test-model/weights.bin").is_file());
        assert_eq!(report.files_skipped.len(), 1);
        assert_eq!(report.bytes_downloaded, 0);
    }

    #[test]
    fn corrupt_target_is_refetched() {
        let temp = tempdir().expect("tempdir");
        let (source, files) = make_source(&[("weights.bin", b"abcdefghij")]);
        let manager = test_manager(temp.path(), files);
        let id = ModelId::new("test-model");
        fs::create_dir_all(temp.path().join("test-model")).expect("model dir");
        fs::write(temp.path().join("test-model/weights.bin"), b"XXXXXXXXXX")
            .expect("corrupt target");

        ModelDownloader::with_source(Box::new(source.clone()))
            .download(&manager, &id, &mut |_| {})
            .expect("refetch");

        assert_eq!(source.starts("mem://weights.bin"), vec![0]);
        assert_eq!(
            fs::read(temp.path().join("test-model/weights.bin")).expect("target"),
            b"abcdefghij"
        );
    }

    #[test]
    fn wrong_size_target_is_refetched() {
        let temp = tempdir().expect("tempdir");
        let (source, files) = make_source(&[("weights.bin", b"abcdefghij")]);
        let manager = test_manager(temp.path(), files);
        let id = ModelId::new("test-model");
        fs::create_dir_all(temp.path().join("test-model")).expect("model dir");
        fs::write(temp.path().join("test-model/weights.bin"), b"abc").expect("short target");

        ModelDownloader::with_source(Box::new(source.clone()))
            .download(&manager, &id, &mut |_| {})
            .expect("refetch");

        assert!(matches!(
            manager.verify(&id).expect("verify"),
            ModelStatus::Verified { .. }
        ));
    }

    #[test]
    fn checksum_mismatch_cleans_part_and_target() {
        let temp = tempdir().expect("tempdir");
        // The manifest digest describes b"abcdefghij", but the source serves a
        // same-length impostor so only the final hash can catch it.
        let (source, mut files) = make_source(&[("weights.bin", b"abcdefghij")]);
        files[0].sha256 = crate::sha256::digest_hex(b"0123456789");
        let manager = test_manager(temp.path(), files);
        let id = ModelId::new("test-model");
        // A previously corrupt target must not survive either.
        fs::create_dir_all(temp.path().join("test-model")).expect("model dir");
        fs::write(temp.path().join("test-model/weights.bin"), b"stale-data").expect("stale");

        let error = ModelDownloader::with_source(Box::new(source.clone()))
            .download(&manager, &id, &mut |_| {})
            .expect_err("checksum mismatch");

        assert!(matches!(error, DownloadError::ChecksumMismatch { .. }));
        assert!(!temp.path().join("test-model/weights.bin").exists());
        assert!(!temp.path().join("test-model/weights.bin.part").exists());
    }

    #[test]
    fn short_stream_keeps_part_then_resume_finishes() {
        let temp = tempdir().expect("tempdir");
        let (mut source, files) = make_source(&[("weights.bin", b"abcdefghij")]);
        let manager = test_manager(temp.path(), files);
        let id = ModelId::new("test-model");
        let url = "mem://weights.bin";
        source.set_serve_len(url, 6);

        let error = ModelDownloader::with_source(Box::new(source.clone()))
            .download(&manager, &id, &mut |_| {})
            .expect_err("short stream");
        assert!(
            matches!(error, DownloadError::ShortStream { received: 6, .. }),
            "unexpected error: {error}"
        );
        let part = temp.path().join("test-model/weights.bin.part");
        assert_eq!(fs::metadata(&part).expect("kept part").len(), 6);

        // The server recovers and the rerun resumes from byte 6.
        source.set_serve_len(url, 10);
        let report = ModelDownloader::with_source(Box::new(source.clone()))
            .download(&manager, &id, &mut |_| {})
            .expect("resumed rerun");
        assert_eq!(source.starts(url), vec![0, 6]);
        assert_eq!(report.bytes_downloaded, 4);
        assert!(matches!(
            manager.verify(&id).expect("verify"),
            ModelStatus::Verified { .. }
        ));
    }

    #[test]
    fn upstream_length_mismatch_is_rejected_before_any_read() {
        let temp = tempdir().expect("tempdir");
        let (mut source, files) = make_source(&[("weights.bin", b"abcdefghij")]);
        source.set_claimed_len("mem://weights.bin", 99);
        let manager = test_manager(temp.path(), files);
        let id = ModelId::new("test-model");

        let error = ModelDownloader::with_source(Box::new(source.clone()))
            .download(&manager, &id, &mut |_| {})
            .expect_err("length mismatch");

        assert!(matches!(error, DownloadError::UpstreamSizeMismatch { .. }));
        assert!(source.starts("mem://weights.bin").is_empty());
        assert!(!temp.path().join("test-model/weights.bin.part").exists());
    }

    #[test]
    fn progress_callback_reports_manifest_totals() {
        let temp = tempdir().expect("tempdir");
        let (source, files) = make_source(&[("a.bin", b"aaa"), ("b.bin", b"bbbbb")]);
        let manager = test_manager(temp.path(), files.clone());
        let id = ModelId::new("test-model");
        let downloader = ModelDownloader::with_source(Box::new(source));

        let mut events = Vec::new();
        downloader
            .download(&manager, &id, &mut |progress| events.push(progress))
            .expect("download");

        assert_eq!(events.len(), 2, "small files report only completions");
        assert_eq!(events[0].file_index, 0);
        assert_eq!(events[0].file_count, 2);
        assert_eq!(events[0].file_name, "a.bin");
        assert_eq!(events[0].file_done, events[0].file_total);
        let last = events.last().expect("events");
        assert_eq!(last.overall_done, last.overall_total);
        assert_eq!(last.overall_total, 8);
        let mut max_seen = 0;
        assert!(events.iter().all(|event| {
            let monotonic = event.overall_done >= max_seen;
            max_seen = event.overall_done;
            monotonic
        }));
    }

    #[test]
    fn progress_throttles_large_streams() {
        let temp = tempdir().expect("tempdir");
        let body = vec![0xAB_u8; 4 * 1024 * 1024];
        let (source, files) = make_source(&[("big.bin", &body)]);
        let manager = test_manager(temp.path(), files.clone());
        let id = ModelId::new("test-model");
        let downloader = ModelDownloader::with_source(Box::new(source));

        let mut events = Vec::new();
        downloader
            .download(&manager, &id, &mut |progress| events.push(progress))
            .expect("download");

        // A per-chunk ticker would emit 64 callbacks for 4 MiB; the throttle
        // collapses that to the step boundary Plus the completion event.
        assert_eq!(events.len(), 2);
        assert_eq!(events[1].overall_done, 4 * 1024 * 1024);
    }

    #[test]
    fn offline_mode_refuses_download() {
        let temp = tempdir().expect("tempdir");
        let (source, files) = make_source(&[("weights.bin", b"abcdefghij")]);
        let mut manager = test_manager(temp.path(), files);
        manager.set_offline(true);
        let id = ModelId::new("test-model");

        let error = ModelDownloader::with_source(Box::new(source.clone()))
            .download(&manager, &id, &mut |_| {})
            .expect_err("offline");

        assert!(matches!(error, DownloadError::Offline));
        assert!(source.starts("mem://weights.bin").is_empty());
        assert!(!temp.path().join("test-model").exists());
    }

    #[test]
    fn builtin_model_reports_nothing_to_download() {
        let manager = ModelManager::new();
        let error = ModelDownloader::new()
            .download(&manager, &ModelId::new("builtin-acoustic"), &mut |_| {})
            .expect_err("builtin");
        assert!(matches!(error, DownloadError::NothingToDownload(_)));
    }

    #[test]
    fn license_gate_blocks_until_accepted() {
        let temp = tempdir().expect("tempdir");
        let (source, files) = make_source(&[("weights.bin", b"abcdefghij")]);
        let mut manager = ModelManager::with_config(ModelManagerConfig {
            storage_dir: temp.path().to_path_buf(),
            offline_mode: false,
        });
        manager.register_manifest(manifest_with(files, false));
        let id = ModelId::new("test-model");
        let downloader = ModelDownloader::with_source(Box::new(source.clone()));

        let error = downloader
            .download(&manager, &id, &mut |_| {})
            .expect_err("license gate");
        assert!(matches!(error, DownloadError::LicenseNotAccepted { .. }));
        assert!(source.starts("mem://weights.bin").is_empty());

        manager.accept_license(&id).expect("accept");
        downloader
            .download(&manager, &id, &mut |_| {})
            .expect("download after accept");
    }

    #[test]
    fn manifest_paths_may_not_escape_the_model_dir() {
        let temp = tempdir().expect("tempdir");
        for evil in ["../evil.bin", "/etc/passwd", ".."] {
            let (source, files) = make_source(&[("weights.bin", b"abcdefghij")]);
            let mut files = files;
            files[0].filename = evil.to_string();
            let manager = test_manager(temp.path(), files);
            let error = ModelDownloader::with_source(Box::new(source.clone()))
                .download(&manager, &ModelId::new("test-model"), &mut |_| {})
                .expect_err("path traversal");
            assert!(matches!(error, DownloadError::InvalidFilename(_)));
        }
    }
}
