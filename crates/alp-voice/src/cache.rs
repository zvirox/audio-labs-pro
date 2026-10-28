//! Speech generation audio caching to avoid redundant re-synthesis.
//!
//! Keys cached audio by cryptographic SHA-256 hash over `(profile_id, text, params)`.

use alp_media::PcmBuffer;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::profile::VoiceParams;
use crate::sha256::digest_hex;

/// In-memory and session audio cache for synthesized speech sentences.
#[derive(Debug, Default)]
pub struct VoiceGenerationCache {
    entries: RwLock<HashMap<String, PcmBuffer>>,
}

impl VoiceGenerationCache {
    /// Create a new empty generation cache.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }

    /// Compute a deterministic 64-character hex key for a synthesis request.
    #[must_use]
    pub fn compute_key(profile_id: &str, text: &str, params: &VoiceParams) -> String {
        let payload = format!(
            "{}:{}:{:.3}:{:.3}:{:.3}:{}",
            profile_id, text, params.speed, params.pitch, params.energy, params.emotion
        );
        digest_hex(payload.as_bytes())
    }

    /// Retrieve cached audio if present.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<PcmBuffer> {
        let lock = self.entries.read().ok()?;
        lock.get(key).cloned()
    }

    /// Store synthesized audio in the cache.
    pub fn put(&self, key: impl Into<String>, buffer: PcmBuffer) {
        if let Ok(mut lock) = self.entries.write() {
            lock.insert(key.into(), buffer);
        }
    }

    /// Number of cached items.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.read().map(|l| l.len()).unwrap_or(0)
    }

    /// Whether the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clear all entries from the cache.
    pub fn clear(&self) {
        if let Ok(mut lock) = self.entries.write() {
            lock.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_stores_and_retrieves_pcm() {
        let cache = VoiceGenerationCache::new();
        let params = VoiceParams::default();
        let key = VoiceGenerationCache::compute_key("aura", "Hello world", &params);

        let buf = PcmBuffer::new(48_000, 1, vec![0.1, 0.2, 0.3]).expect("buf");
        assert!(cache.get(&key).is_none());

        cache.put(&key, buf.clone());
        let hit = cache.get(&key);
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().samples(), buf.samples());
    }

    #[test]
    fn different_params_generate_distinct_keys() {
        let p1 = VoiceParams {
            speed: 1.0,
            ..Default::default()
        };
        let p2 = VoiceParams {
            speed: 1.2,
            ..Default::default()
        };

        let k1 = VoiceGenerationCache::compute_key("aura", "Test", &p1);
        let k2 = VoiceGenerationCache::compute_key("aura", "Test", &p2);

        assert_ne!(k1, k2);
    }
}
