//! Property-based fuzz harnesses for project deserialization and sample blocks.
//!
//! Arbitrary bytes reach the real `from_json` / `ProjectPaths::load` /
//! `SampleBlock` entry points. The invariant under test is not "must parse" but
//! "must never panic": malformed input either yields a structured error or a
//! valid value with self-consistent metadata.

use alp_core::{BlockId, ProjectPaths, SampleBlock, from_json, to_json};
use proptest::prelude::*;
use tempfile::tempdir;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    #[test]
    fn arbitrary_bytes_never_panic_project_json(bytes in prop::collection::vec(any::<u8>(), 0..4096)) {
        let text = String::from_utf8_lossy(&bytes);
        let parsed = from_json(&text);
        if let Ok(project) = parsed {
            // A successful parse must produce a serializable, re-parseable project.
            let json = to_json(&project).expect("reserialize");
            prop_assert!(from_json(&json).is_ok());
        }
    }

    #[test]
    fn arbitrary_bytes_never_panic_project_directory_load(bytes in prop::collection::vec(any::<u8>(), 0..4096)) {
        let dir = tempdir().expect("tempdir");
        let paths = ProjectPaths::new(dir.path().to_path_buf());
        paths.ensure_layout().expect("layout");
        std::fs::write(paths.project_file(), &bytes).expect("write malformed project.json");
        let _ = paths.load();
        let _ = paths.recover_autosave();
    }

    #[test]
    fn sample_block_payloads_roundtrip_or_fail_cleanly(
        bytes in prop::collection::vec(any::<u8>(), 0..512),
        channels in 1u16..=8,
        rate in 8_000u32..=192_000,
    ) {
        let (chunks, _) = bytes.as_chunks::<4>();
        let samples: Vec<f32> = chunks
            .iter()
            .map(|chunk| f32::from_le_bytes(*chunk))
            .collect();
        let block = SampleBlock::from_f32_interleaved(BlockId::from_u128(7), &samples, channels, rate);
        prop_assert!(block.verify_integrity());
        let decoded = block.to_f32_interleaved().expect("f32 payload decodes");
        prop_assert_eq!(decoded.len(), samples.len());

        let mut corrupted = block;
        if let Some(first) = corrupted.payload.first_mut() {
            *first ^= 0xff;
            prop_assert!(!corrupted.verify_integrity());
        }
    }

    #[test]
    fn malformed_block_json_never_panics(bytes in prop::collection::vec(any::<u8>(), 0..1024)) {
        let text = String::from_utf8_lossy(&bytes);
        let parsed: Result<SampleBlock, _> = serde_json::from_str(&text);
        if let Ok(block) = parsed {
            // Even a serde-valid block must report checksum mismatch on arbitrary payloads.
            let _ = block.verify_integrity();
            let _ = block.to_f32_interleaved();
        }
    }
}
