//! Block-based audio sample storage and schemas.
//!
//! Large multitrack projects split audio into fixed-size sample blocks (e.g. 64K
//! frames) to enable instantaneous non-destructive slicing, shared clip
//! references, fast undo/redo rollback, and crash-resilient SQLite WAL storage.

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::CoreError;

/// Standard frame capacity per sample block (approx 1.36 seconds at 48 kHz).
pub const DEFAULT_BLOCK_FRAMES: u32 = 65_536;

/// Stable block identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BlockId(pub Uuid);

impl BlockId {
    /// Generate a new random block identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Construct a deterministic block identifier from a 128-bit integer.
    #[must_use]
    pub fn from_u128(v: u128) -> Self {
        Self(Uuid::from_u128(v))
    }
}

impl Default for BlockId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Supported raw sample representations inside a block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SampleFormat {
    /// 32-bit floating point PCM.
    #[default]
    F32,
    /// 24-bit signed integer PCM.
    I24,
    /// 16-bit signed integer PCM.
    I16,
}

/// Compression encoding applied to stored block payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BlockCompression {
    /// Uncompressed raw PCM bytes.
    #[default]
    None,
    /// Lossless FLAC encoding.
    Flac,
    /// High-speed LZ4 / Zstandard compression.
    Zstd,
}

/// Metadata header describing sample block properties.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Number of audio channels (e.g. 1 for mono, 2 for stereo).
    pub channels: u16,
    /// Sample rate in Hertz (e.g. 48,000).
    pub sample_rate: u32,
    /// Number of frames stored in this block (at most [`DEFAULT_BLOCK_FRAMES`]).
    pub frame_count: u32,
    /// PCM representation.
    pub format: SampleFormat,
    /// Compression format applied to the raw bytes.
    pub compression: BlockCompression,
    /// CRC32 or Adler32 checksum of data bytes for integrity verification.
    pub checksum: u32,
}

/// A chunk of audio samples stored in memory or on disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SampleBlock {
    /// Unique identifier.
    pub id: BlockId,
    /// Header metadata.
    pub header: BlockHeader,
    /// Raw or compressed byte payload.
    pub payload: Vec<u8>,
}

impl SampleBlock {
    /// Construct a mono or stereo 32-bit float uncompressed block.
    #[must_use]
    pub fn from_f32_interleaved(
        id: BlockId,
        samples: &[f32],
        channels: u16,
        sample_rate: u32,
    ) -> Self {
        let frame_count = (samples.len() / usize::from(channels.max(1))) as u32;
        let mut payload = Vec::with_capacity(samples.len() * 4);
        for &sample in samples {
            payload.extend_from_slice(&sample.to_le_bytes());
        }

        let checksum = simple_checksum(&payload);

        Self {
            id,
            header: BlockHeader {
                channels,
                sample_rate,
                frame_count,
                format: SampleFormat::F32,
                compression: BlockCompression::None,
                checksum,
            },
            payload,
        }
    }

    /// Decode payload into interleaved 32-bit float samples.
    pub fn to_f32_interleaved(&self) -> Result<Vec<f32>, CoreError> {
        if self.header.compression != BlockCompression::None {
            return Err(CoreError::validation(
                "compressed block decoding requires codec module",
            ));
        }

        if self.header.format != SampleFormat::F32 {
            return Err(CoreError::validation("non-f32 format decoding unsupported"));
        }

        if !self.payload.len().is_multiple_of(4) {
            return Err(CoreError::validation(
                "payload size is not aligned to 4 bytes",
            ));
        }

        let count = self.payload.len() / 4;
        let mut samples = Vec::with_capacity(count);
        let (chunks, _) = self.payload.as_chunks::<4>();
        for chunk in chunks {
            samples.push(f32::from_le_bytes(*chunk));
        }

        Ok(samples)
    }

    /// Verify payload integrity against the stored checksum.
    #[must_use]
    pub fn verify_integrity(&self) -> bool {
        self.header.checksum == simple_checksum(&self.payload)
    }
}

/// Fast checksum for sample block payload verification.
fn simple_checksum(bytes: &[u8]) -> u32 {
    let mut sum: u32 = 0;
    for &b in bytes {
        sum = sum.wrapping_mul(31).wrapping_add(u32::from(b));
    }
    sum
}

/// SQLite schema definition for block-based project storage.
pub struct BlockStoreSchema;

impl BlockStoreSchema {
    /// SQL table definitions for SQLite project databases.
    pub const SCHEMA_SQL: &'static str = r#"
        CREATE TABLE IF NOT EXISTS project_metadata (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS sample_blocks (
            block_id TEXT PRIMARY KEY,
            channels INTEGER NOT NULL,
            sample_rate INTEGER NOT NULL,
            frame_count INTEGER NOT NULL,
            format TEXT NOT NULL,
            compression TEXT NOT NULL,
            checksum INTEGER NOT NULL,
            payload BLOB NOT NULL
        );

        CREATE TABLE IF NOT EXISTS clip_block_map (
            clip_id TEXT NOT NULL,
            block_index INTEGER NOT NULL,
            block_id TEXT NOT NULL,
            PRIMARY KEY (clip_id, block_index),
            FOREIGN KEY(block_id) REFERENCES sample_blocks(block_id)
        );

        CREATE INDEX IF NOT EXISTS idx_clip_blocks ON clip_block_map(clip_id);
    "#;
}

/// In-memory cache and storage for sample blocks.
#[derive(Debug, Default)]
pub struct MemoryBlockStore {
    blocks: HashMap<BlockId, SampleBlock>,
}

impl MemoryBlockStore {
    /// Create an empty in-memory block store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
        }
    }

    /// Insert a block into the store.
    pub fn insert(&mut self, block: SampleBlock) {
        self.blocks.insert(block.id, block);
    }

    /// Retrieve a reference to a block by ID.
    #[must_use]
    pub fn get(&self, id: &BlockId) -> Option<&SampleBlock> {
        self.blocks.get(id)
    }

    /// Remove a block from the store.
    pub fn remove(&mut self, id: &BlockId) -> Option<SampleBlock> {
        self.blocks.remove(id)
    }

    /// Total number of stored blocks.
    #[must_use]
    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    /// Whether the store contains no blocks.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// Total byte size of all stored payloads.
    #[must_use]
    pub fn total_payload_bytes(&self) -> usize {
        self.blocks.values().map(|b| b.payload.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_block_round_trip() {
        let original_samples: Vec<f32> = vec![0.0, 0.5, -0.5, 0.25, -0.25, 0.125];
        let block_id = BlockId::new();
        let block = SampleBlock::from_f32_interleaved(block_id, &original_samples, 1, 48_000);

        assert!(block.verify_integrity());
        assert_eq!(block.header.frame_count, 6);
        assert_eq!(block.header.channels, 1);
        assert_eq!(block.header.sample_rate, 48_000);

        let decoded = block.to_f32_interleaved().unwrap();
        assert_eq!(decoded, original_samples);
    }

    #[test]
    fn memory_block_store_operations() {
        let mut store = MemoryBlockStore::new();
        assert!(store.is_empty());

        let samples = vec![0.1f32; 100];
        let block = SampleBlock::from_f32_interleaved(BlockId::new(), &samples, 2, 48_000);
        let id = block.id;

        store.insert(block);
        assert_eq!(store.len(), 1);
        assert_eq!(store.total_payload_bytes(), 400);

        assert!(store.get(&id).is_some());
        assert!(store.remove(&id).is_some());
        assert!(store.is_empty());
    }
}
