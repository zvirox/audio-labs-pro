# Live Transport Bridge Results

Date: 2026-11-26

## Deliverables

- `alp-session::ai::transport_control`: intercepts `transport_*` tool names on
  the UI tick thread before the scratch worker and revision-commit path, and
  drives the injected `TransportController` against the live `EditSession`.
- `AiControlManager::set_transport` and a one-time injection in
  `alp-desktop/src/main.rs`; no protocol or dependency changes were needed —
  the existing opaque `ControlRequest::Mcp` envelope carries the calls.
- `alp-mcp`: seven tools (`transport_state`, `transport_play`,
  `transport_pause`, `transport_stop`, `transport_seek`, `transport_set_volume`,
  `transport_set_loop`) bring the advertised surface to 86. Transport tools are
  excluded from `expected_revision` decoration because they never touch the
  project snapshot.
- Headless fallback in `transport_dispatch.rs` returns an explicit
  "requires a live desktop attach" error; no fake `TransportState` is produced.
- Attached round-trip test in `crates/alp-session/tests/attached_transport.rs`
  drives play/state/seek/loop/pause over a real Unix control socket.

## Verification

- `cargo test -p alp-session --test attached_transport`: 1 passed; the live
  controller changed state and the project undo stack stayed unchanged.
- `bash scripts/mcp-eval.sh`: **78/78 checks passed (100%)**, including four new
  Suite 11 headless-honesty checks.
- `cargo fmt --all -- --check`, workspace clippy (`-D warnings`), workspace
  tests, all four guards, `check:linear`, frontend build/test and `check-docs`
  passed.

## Honest limits

- Live control requires `alp-mcp --attach` plus a running desktop app with AI
  control enabled; the headless binary deliberately refuses these tools.
- Transport actions are live device state, not project edits: they are not
  undoable through history tools and carry no `expected_revision`.
- Attached mode requires a saved project for the full MCP surface, but the
  transport interception itself works before that guard.
- No tag, push or version bump was made.

## Acceptance harness

`bash scripts/mcp-eval.sh` (headless honesty) and
`cargo test -p alp-session --test attached_transport` (live control).
