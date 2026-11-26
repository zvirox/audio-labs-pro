# Extended MCP Coverage Results — Dubbing & Repair

Date: 2026-11-26

## Deliverables

- `alp-api::dubbing_ops`: sandbox-checked `dubbing_fit_rate` (WSOLA via
  `alp_voice::fit_take`) and `dubbing_remux` (ffmpeg stream-copy) wrappers.
- `alp-api::repair_ops`: offline `dsp_spectral_repair` (notch/harmonic) and
  `dsp_time_stretch` wrappers over the existing `alp_dsp` effects.
- `alp-mcp`: four new Export-tier tools, bringing the advertised surface from
  75 to 79. Dispatch lives in `coverage_ext_dispatch.rs` so `dispatch.rs`
  remains under the 1500-line guard.
- `scripts/mcp_eval.py` Suite 10 exercises all four tools over real stdio;
  scenario S03 and `main.rs` assert the 79-tool surface.

## Verification

- `cargo test -p alp-api dubbing_ops`: 3 passed.
- `cargo test -p alp-api repair_ops`: 4 passed.
- `cargo test -p alp-mcp`: 6 + 26 tests passed.
- `bash scripts/mcp-eval.sh`: **74/74 checks passed (100%)**, including five
  new Suite 10 checks; report written to `target/mcp_eval_results.json`.
- `cargo fmt --all -- --check`, workspace clippy (`-D warnings`), workspace
  tests, all four guards, `check:linear` and frontend build/test passed.

## Honest limits

- Live transport tools (play/pause/seek) are not exposed: the MCP server has no
  transport handle, and adding one requires control-bridge plumbing in
  `alp-session`.
- Demucs stem separation is not exposed: `alp-mcp` cannot depend on
  `alp-infer`, and no model weights are bundled.
- `dubbing_remux` requires external `ffmpeg`/`ffprobe`; Suite 10 verifies the
  error path for a missing source video, not a successful video remux.
- All new tools are offline file-to-file operations; they do not modify the
  active project timeline.

## Acceptance harness

`bash scripts/mcp-eval.sh`
