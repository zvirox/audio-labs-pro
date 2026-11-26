# Phase 15 Results — Audiobook Mastering & Native M4B Packaging

Date: 2026-11-26

## Deliverables

- `alp-dsp::true_peak`: quarter-sample Catmull-Rom inter-sample peak estimation with tests.
- `alp-api::acx_qa`: per-chapter RMS / true-peak / noise-floor validation against ACX targets, JSON DTOs, and tests.
- `alp-media::m4b`: pure-Rust AAC-LC encoder plus ISO-BMFF writer with injected `chpl`, `covr`, and `meta/ilst` boxes; atomic output; test verifies markers in the exported file.
- `alp-session::audiobook_mastering`: decode-from-disk bridge exposing ACX analysis and M4B export to Tauri.
- `alp-desktop` commands `audiobook_acx_analyze` and `audiobook_export_m4b`.
- React Stories workspace controls: master/cover paths, ACX check, M4B export, pass/fail report rows using the locked Linear tokens.

## Dependencies

Added MIT-licensed crates: `oxideav-aac` 0.1.7 (AAC-LC encoder), `mp4` 0.14.0 (ISO-BMFF writer), `bytes` 1.x. Recorded in `THIRD_PARTY.md`; no FDK or non-permissive codec was introduced.

## Verification

`cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo fmt --all -- --check`, all four repository guard scripts, `npm --prefix frontend run check:linear`, `npm --prefix frontend run build`, and `npm --prefix frontend test` all passed. The M4B test asserts the destination file contains `chpl`, `covr`, and `ilst` markers.

## Known limits

- RMS is measured in dBFS, not gated LUFS.
- True peak is a quarter-sample cubic approximation, not the full BS.1770-4 reconstruction filter.
- M4B supports mono/stereo AAC-LC only; cover art is JPEG or PNG.

## Acceptance harness

`bash scripts/phase15-demo.sh`
