# `alp-voice` / `cache`

## Purpose

Provides in-memory caching of synthesized audio buffers keyed by cryptographic SHA-256 hash of `(profile_id, text, params)` to prevent redundant speech synthesis passes.

## Public API

- `VoiceGenerationCache`: `new`, `compute_key`, `get`, `put`, `len`, `is_empty`, `clear`

## Data flow

1. When a synthesis request is submitted, `compute_key` calculates a SHA-256 digest of the profile ID, input text, and vocal parameters.
2. `get` checks for cached `PcmBuffer`.
3. If absent, the engine synthesizes the line and calls `put` to cache the buffer.

## Allowed dependencies

- `alp-media::PcmBuffer`
- `alp-voice::{VoiceParams, sha256::digest_hex}`
- Standard library (`std::sync::RwLock`, `std::collections::HashMap`)

## Threading / real-time notes

Thread-safe using `std::sync::RwLock` for concurrent read access from multiple worker threads.

## Error handling

Cache lookups return `None` on cache misses or lock poison recovery without throwing errors.

## Testing

Unit tests in `cache.rs` verify buffer insertion, retrieval, and that varying parameters generate distinct cache keys.

## Known limits

- In-memory per process session; persistent multi-session disk caching can be added.
