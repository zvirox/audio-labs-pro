//! Property-based fuzz harnesses for the media decoder entry points.
//!
//! Arbitrary bytes are written to disk and handed to `probe`, `decode` and
//! `decode_path`. The invariant is that a malformed container returns a
//! structured `MediaError` (or a self-consistent valid result) without ever
//! panicking — decoders must survive hostile input on an import worker thread.

use alp_media::{decode, decode_path, probe, write_test_tone};
use proptest::prelude::*;
use tempfile::NamedTempFile;

use std::io::Write;

fn with_bytes<R>(bytes: &[u8], check: impl FnOnce(&std::path::Path) -> R) -> R {
    let mut file = NamedTempFile::new().expect("temp file");
    file.write_all(bytes).expect("write fuzz input");
    file.flush().expect("flush fuzz input");
    check(file.path())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn arbitrary_bytes_never_panic_probe(bytes in prop::collection::vec(any::<u8>(), 0..4096)) {
        with_bytes(&bytes, |path| -> Result<(), TestCaseError> {
            if let Ok(info) = probe(path) {
                prop_assert!(info.sample_rate > 0);
                prop_assert!(info.channels > 0);
            }
            Ok(())
        })?;
    }

    #[test]
    fn arbitrary_bytes_never_panic_full_decode(bytes in prop::collection::vec(any::<u8>(), 0..4096)) {
        with_bytes(&bytes, |path| -> Result<(), TestCaseError> {
            if let Ok(pcm) = decode(path) {
                prop_assert!(pcm.sample_rate() > 0);
                prop_assert!(pcm.channels() > 0);
                prop_assert_eq!(
                    pcm.samples().len(),
                    pcm.frames() as usize * usize::from(pcm.channels())
                );
            }
            Ok(())
        })?;
    }

    #[test]
    fn arbitrary_bytes_never_panic_streaming_decode(bytes in prop::collection::vec(any::<u8>(), 0..4096)) {
        with_bytes(&bytes, |path| -> Result<(), TestCaseError> {
            let mut chunk_dims = Vec::new();
            if let Ok(info) = decode_path(path, |samples, channels| {
                chunk_dims.push((channels, samples.len()));
                Ok(())
            }) {
                prop_assert!(info.sample_rate > 0);
            }
            for (channels, len) in chunk_dims {
                prop_assert!(channels > 0);
                prop_assert_eq!(len % usize::from(channels), 0);
            }
            Ok(())
        })?;
    }
}

#[test]
fn truncated_valid_wav_is_rejected_without_panicking() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("tone.wav");
    write_test_tone(&path, 0.05, 48_000, 2, 440.0, 0.2).expect("write seed tone");
    let bytes = std::fs::read(&path).expect("read seed tone");
    assert!(bytes.len() > 64);

    // Truncate at several boundaries inside the RIFF header and data chunk.
    for keep in [0, 4, 12, 36, 44, bytes.len() / 2, bytes.len() - 1] {
        with_bytes(&bytes[..keep], |probe_path| {
            let _ = probe(probe_path);
            let _ = decode(probe_path);
        });
    }
}
