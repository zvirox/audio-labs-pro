# `alp-core::block`

Block-based audio sample chunking, metadata headers, and storage schema.

## Purpose

Large multitrack editing projects split audio streams into fixed-size sample blocks
(e.g., 65,536 frames) to enable instantaneous non-destructive clip cutting, shared block
references across clips, fast undo/redo rollback, and crash-resilient storage in SQLite WAL
databases.

## Public API

```rust
pub const DEFAULT_BLOCK_FRAMES: u32 = 65_536;

pub struct BlockId(pub Uuid);

pub enum SampleFormat {
    F32,
    I24,
    I16,
}

pub enum BlockCompression {
    None,
    Flac,
    Zstd,
}

pub struct BlockHeader {
    pub channels: u16,
    pub sample_rate: u32,
    pub frame_count: u32,
    pub format: SampleFormat,
    pub compression: BlockCompression,
    pub checksum: u32,
}

pub struct SampleBlock {
    pub id: BlockId,
    pub header: BlockHeader,
    pub payload: Vec<u8>,
}

pub struct BlockStoreSchema;

pub struct MemoryBlockStore;
```

## Data flow

Incoming audio recordings and imported files are chunked into `SampleBlock`s. Clips
maintain an ordered index of block references. During playback, blocks are retrieved
from memory or disk caches to stream into audio buffers.

## Allowed dependencies

- `serde` for serialization.
- `uuid` for unique block identifiers.
- `crate::error::CoreError`.
- Core Rust standard library (`std`).

## Threading / real-time notes

Block data structures are immutable once created. Storage queries in real-time callbacks
use pre-buffered lock-free caches so disk reads never block audio processing.

## Error handling

Methods return `Result<T, CoreError>` on corrupt payloads, invalid alignments, or
unsupported compression codecs. Checksum verification detects payload corruption.

## Testing

Unit tests in `crates/alp-core/src/block.rs` verify:
- Interleaved 32-bit float sample round-tripping through byte payloads.
- Checksum integrity validation.
- In-memory block store insertions, retrievals, and deletions.

## Known limits

Direct disk persistence in this initial phase is modeled via `MemoryBlockStore` and
`BlockStoreSchema` SQL definitions; SQLite database binding lands in subsequent phases.
