# Phase 18 Results — Long-Horizon Verification & Fuzzing

Date: 2026-11-26

## Deliverables

- `scripts/mcp_eval_scenarios.py`: 25 named MCP scenarios (S01–S25) covering
  handshake/discovery, tiers, sandbox, tool catalog, full workflow, single-entry
  undo, checkpoint restore, loudness/quality analysis, export, **deep rollback
  equality** against the baseline snapshot, and **deterministic convergence**
  across two fresh workspaces.
- `scripts/mcp_eval.py`: now runs the legacy 8 suites plus the 25 scenarios and
  writes `target/mcp_eval_results.json` when a report path is supplied.
  `scripts/mcp-eval.sh` passes that path automatically.
- `crates/alp-session/tests/long_horizon_soak.rs`: short CI soak (1,000 ticks)
  and an ignored 86,400-tick (24-hour-equivalent) soak asserting zero dropped
  callbacks, monotonic position, live transport and bounded RSS growth via
  `/proc/self/status`.
- `crates/alp-core/tests/fuzz_project.rs`: proptest harnesses for arbitrary
  bytes through `from_json`, `ProjectPaths::load`/`recover_autosave`, malformed
  `SampleBlock` JSON, and `SampleBlock` checksum/roundtrip invariants.
- `crates/alp-media/tests/fuzz_decode.rs`: proptest harnesses driving arbitrary
  bytes through `probe`, `decode` and `decode_path`, plus truncated valid-WAV
  cases. Added `proptest` as an `alp-media` dev-dependency.

## Verification

- `bash scripts/mcp-eval.sh`: **62/62 checks passed (100%)**, report written to
  `target/mcp_eval_results.json`.
- `cargo test -p alp-core --test fuzz_project`: 4 passed.
- `cargo test -p alp-media --test fuzz_decode`: 4 passed.
- `cargo test -p alp-session --test long_horizon_soak`: 1 passed, 1 ignored.
- Ignored 24h soak executed explicitly once: passed (86,400 callbacks, no
  dropped frames, RSS growth within the 256 MB guard).
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked
  -- -D warnings`, `cargo test --workspace --locked`, all four guard scripts,
  `check:linear` and frontend build/test passed.
- `check-docs`: 168 documents, all required sections present.

## Honest limits

- The 25 scenarios are a named regression/convergence suite, not a load or
  timing benchmark; they assert state convergence and rollback equality, not
  latency percentiles.
- The "24-hour" soak accelerates monitor callbacks, not audio samples: the
  headless transport uses a wall-clock virtual clock, so 86,400 ticks exercise
  callback durability and memory stability rather than 24 hours of real-time
  audio. Hardware drop-frame counting remains in
  `scripts/phase3-demo.sh --soak --minutes N`.
- No SQLite engine exists in this workspace; "SQLite deserializer fuzzing"
  targets the `SampleBlock`/`BlockHeader` serde and checksum layer instead of a
  real SQL parser. Project files are a `*.alp/` directory containing
  `project.json`, not a single `.alpproj` file.
- Fuzz harnesses use `proptest` (already a workspace dependency), not
  `cargo-fuzz`; no new runtime dependency was introduced.

## Acceptance harness

`bash scripts/phase18-demo.sh` (add `--soak --hours 24` to include the
long-horizon soak).
