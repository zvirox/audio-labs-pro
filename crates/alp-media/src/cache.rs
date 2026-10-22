//! On-disk peak pyramid cache.
//!
//! Cache files follow ADR 0009 and live under a caller-provided directory (the
//! project's `peaks/` folder in normal use). They are derived data: a corrupt or
//! stale cache is deleted and rebuilt, never treated as authoritative.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use alp_core::write_atomic;

use crate::decode;
use crate::error::MediaError;
use crate::peaks::{BASE_BUCKET, CACHE_MAGIC, CACHE_VERSION, PeakLevel, PeakPyramid};

/// Whether a pyramid came from disk or was rebuilt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheStatus {
    /// Loaded from an existing cache file.
    Hit,
    /// Rebuilt and written to disk.
    Miss,
}

/// Peak cache rooted at a directory.
#[derive(Debug, Clone)]
pub struct PeakCache {
    root: PathBuf,
}

impl PeakCache {
    /// Use `root` as the cache directory.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Cache directory.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Cache path for a source at a known sample rate.
    pub fn entry_path(&self, source: &Path, sample_rate: u32) -> Result<PathBuf, MediaError> {
        let metadata = std::fs::metadata(source)?;
        let fingerprint = Fingerprint {
            size: metadata.len(),
            modified_ns: modified_ns(&metadata),
            sample_rate,
        };
        let stem = source
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("source");
        let safe: String = stem
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                    character
                } else {
                    '_'
                }
            })
            .take(48)
            .collect();
        Ok(self
            .root
            .join(format!("{safe}-{:016x}.peaks", fingerprint.hash())))
    }

    /// Load a pyramid from disk without touching the source.
    pub fn load(&self, cache_path: &Path) -> Result<PeakPyramid, MediaError> {
        read_cache(cache_path)
    }

    /// Load a valid cache entry or rebuild it from the source.
    pub fn load_or_build(
        &self,
        source: &Path,
    ) -> Result<(PeakPyramid, CacheStatus, PathBuf), MediaError> {
        let info = decode::probe(source)?;
        let cache_path = self.entry_path(source, info.sample_rate)?;
        if let Ok(pyramid) = read_cache(&cache_path) {
            let frames_match = info.frames.is_none_or(|frames| frames == pyramid.frames());
            if pyramid.sample_rate() == info.sample_rate && frames_match {
                return Ok((pyramid, CacheStatus::Hit, cache_path));
            }
        }

        let pyramid = PeakPyramid::build_from_path(source)?;
        let cache_path = self.entry_path(source, pyramid.sample_rate())?;
        write_cache(&cache_path, &pyramid)?;
        Ok((pyramid, CacheStatus::Miss, cache_path))
    }

    /// Write a pyramid for `source`, returning the cache path.
    pub fn store(&self, source: &Path, pyramid: &PeakPyramid) -> Result<PathBuf, MediaError> {
        let cache_path = self.entry_path(source, pyramid.sample_rate())?;
        write_cache(&cache_path, pyramid)?;
        Ok(cache_path)
    }
}

#[derive(Debug)]
struct Fingerprint {
    size: u64,
    modified_ns: u128,
    sample_rate: u32,
}

impl Fingerprint {
    fn hash(&self) -> u64 {
        // FNV-1a over the stable fields. The cache version is part of the format
        // header, so a format change invalidates entries without path surgery.
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        for byte in self
            .size
            .to_le_bytes()
            .into_iter()
            .chain(self.modified_ns.to_le_bytes())
            .chain(self.sample_rate.to_le_bytes())
        {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash
    }
}

fn modified_ns(metadata: &std::fs::Metadata) -> u128 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_nanos())
}

fn write_cache(path: &Path, pyramid: &PeakPyramid) -> Result<(), MediaError> {
    let bytes = encode(pyramid);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    write_atomic(path, &bytes)?;
    Ok(())
}

fn encode(pyramid: &PeakPyramid) -> Vec<u8> {
    let pair_bytes: usize = pyramid.levels().iter().map(|level| level.len() * 4).sum();
    let mut bytes = Vec::with_capacity(28 + pyramid.levels().len() * 12 + pair_bytes);
    bytes.extend_from_slice(CACHE_MAGIC);
    bytes.extend_from_slice(&CACHE_VERSION.to_le_bytes());
    bytes.extend_from_slice(&pyramid.sample_rate().to_le_bytes());
    bytes.extend_from_slice(&pyramid.frames().to_le_bytes());
    bytes.extend_from_slice(&(pyramid.levels().len() as u32).to_le_bytes());
    for level in pyramid.levels() {
        bytes.extend_from_slice(&level.bucket_samples().to_le_bytes());
        bytes.extend_from_slice(&(level.len() as u64).to_le_bytes());
        for (min, max) in level.pairs() {
            bytes.extend_from_slice(&min.to_le_bytes());
            bytes.extend_from_slice(&max.to_le_bytes());
        }
    }
    bytes
}

fn read_cache(path: &Path) -> Result<PeakPyramid, MediaError> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut magic = [0u8; 8];
    reader.read_exact(&mut magic)?;
    if &magic != CACHE_MAGIC {
        return Err(MediaError::InvalidCache("bad magic".into()));
    }
    let version = read_u32(&mut reader)?;
    if version != CACHE_VERSION {
        return Err(MediaError::InvalidCache(format!(
            "cache version {version}, expected {CACHE_VERSION}"
        )));
    }
    let sample_rate = read_u32(&mut reader)?;
    let frames = read_u64(&mut reader)?;
    let level_count = read_u32(&mut reader)?;
    if !(1..=8).contains(&level_count) {
        return Err(MediaError::InvalidCache("level count out of range".into()));
    }

    let mut levels = Vec::with_capacity(level_count as usize);
    for index in 0..level_count {
        let bucket_samples = read_u32(&mut reader)?;
        let expected_bucket = BASE_BUCKET.saturating_mul(4u32.saturating_pow(index));
        if bucket_samples != expected_bucket {
            return Err(MediaError::InvalidCache("unexpected bucket size".into()));
        }
        let count = read_u64(&mut reader)?;
        let maximum = frames / u64::from(bucket_samples) + 1;
        if count == 0 || count > maximum {
            return Err(MediaError::InvalidCache("bucket count out of range".into()));
        }
        let count = usize::try_from(count)
            .map_err(|_| MediaError::InvalidCache("bucket count too large".into()))?;
        let mut pairs = Vec::with_capacity(count);
        for _ in 0..count {
            let min = read_i16(&mut reader)?;
            let max = read_i16(&mut reader)?;
            pairs.push((min, max));
        }
        levels.push(PeakLevel {
            bucket_samples,
            pairs,
        });
    }

    PeakPyramid::from_parts(sample_rate, frames, levels)
}

fn read_u32(reader: &mut impl Read) -> Result<u32, MediaError> {
    let mut bytes = [0u8; 4];
    reader.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64(reader: &mut impl Read) -> Result<u64, MediaError> {
    let mut bytes = [0u8; 8];
    reader.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

fn read_i16(reader: &mut impl Read) -> Result<i16, MediaError> {
    let mut bytes = [0u8; 2];
    reader.read_exact(&mut bytes)?;
    Ok(i16::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synthetic::write_test_tone;

    #[test]
    fn second_load_is_a_cache_hit() {
        let dir = tempfile::tempdir().expect("tempdir");
        let source = dir.path().join("tone.wav");
        write_test_tone(&source, 0.5, 48_000, 1, 440.0, 0.2).expect("tone");
        let cache = PeakCache::new(dir.path().join("peaks"));

        let (first, status, path) = cache.load_or_build(&source).expect("build");
        assert_eq!(status, CacheStatus::Miss);
        assert!(path.is_file());

        let (second, status, _) = cache.load_or_build(&source).expect("load");
        assert_eq!(status, CacheStatus::Hit);
        assert_eq!(first, second);
    }

    #[test]
    fn corrupt_cache_is_rebuilt() {
        let dir = tempfile::tempdir().expect("tempdir");
        let source = dir.path().join("tone.wav");
        write_test_tone(&source, 0.2, 48_000, 1, 440.0, 0.2).expect("tone");
        let cache = PeakCache::new(dir.path().join("peaks"));
        let (_, _, path) = cache.load_or_build(&source).expect("build");
        std::fs::write(&path, b"corrupt").expect("corrupt");
        let (_, status, _) = cache.load_or_build(&source).expect("rebuild");
        assert_eq!(status, CacheStatus::Miss);
    }
}
