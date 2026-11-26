# Phase 19 Results — Release Readiness & MCP Coverage

Date: 2026-11-26

## Deliverables

- `alp-api::studio_ops`: wrappers for EQ frequency response, ACX chapter QA,
  native M4B export, marker removal, loop range and clip trim. File paths are
  sandbox-validated and timeline edits go through audited `alp_core` commands.
- `alp-mcp`: six new tools (`effects_response`, `acx_qa_analyze`,
  `audiobook_export_m4b`, `marker_remove`, `loop_range_set`, `clip_trim`),
  bringing the advertised surface from 69 to 75. Dispatch lives in
  `studio_dispatch.rs` so `dispatch.rs` stays under the 1500-line guard.
- `scripts/mcp_eval.py` Suite 9 exercises all six tools over real stdio;
  scenario S03 and `main.rs` assert the 75-tool surface.
- Release config parity: `release.yml` builds `alp-mcp` and `nfpm.yaml`
  installs `/usr/bin/alp-mcp`.
- `scripts/phase19-release-check.sh`: local nfpm dry-run for deb/rpm/archlinux
  plus SHA256SUMS generation and verification. No tag, push or signature.
- Documentation drift fixed: README/docs no longer advertise ECharts, and
  `docs/release-audit.md` matches the `v0.1.0-beta.3` baseline.

## Verification

- `cargo test -p alp-api studio_ops`: 4 passed (real WAV → ACX report, real M4B
  export, marker/loop undo, EQ unity check).
- `cargo test -p alp-mcp`: 6 + 26 tests passed.
- `bash scripts/mcp-eval.sh`: **69/69 checks passed (100%)**, including seven
  new Suite 9 checks; report written to `target/mcp_eval_results.json`.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked
  -- -D warnings`, `cargo test --workspace --locked`, all four guards,
  `check:linear`, frontend build/test and `check-docs` passed.
- Release binaries built with `cargo build --release --locked -p alp-desktop
  -p alp-cli -p alp-infer -p alp-mcp`, then
  `bash scripts/phase19-release-check.sh`: nfpm produced `.deb`, `.rpm` and
  `.pkg.tar.zst`, the deb was verified to ship `/usr/bin/alp-mcp`, and
  `sha256sum -c` verified `SHA256SUMS`.

## Honest limits

- No version bump to `v1.0.0-rc.1` was made; the public baseline remains
  `v0.1.0-beta.3`. Version promotion and GPG signing are release-cut steps
  delegated to the external reusable workflow.
- Demucs stem separation, insert-rack atomics and VST3 editor instantiation are
  still unavailable through MCP: they are model-gated, live-engine-state, or
  intentionally unsupported in this build.
- `clip_trim` takes frame indices rather than seconds so source-rate ambiguity
  cannot silently corrupt an edit.
- The external `zvirox/release-workflows` reusable workflow is not vendored
  here; CI parity for its checksum/signing behavior remains unverified offline.

## Acceptance harness

`bash scripts/phase19-release-check.sh --build` (add
`--outdir /path/to/dir` to choose the artifact directory).
