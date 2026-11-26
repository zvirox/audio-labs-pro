#!/usr/bin/env bash
# Phase 18 acceptance demo: 25-scenario MCP eval, fuzz harnesses, playback soak.
# Use `--soak --hours 24` to additionally run the long-horizon ignored test.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

SOAK_HOURS=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --soak)
      SOAK_HOURS=24
      shift
      ;;
    --hours)
      SOAK_HOURS="${2:?--hours requires a value}"
      shift 2
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

echo "== [1/5] Repository guards =="
bash scripts/check-file-length.sh
bash scripts/check-deps.sh
bash scripts/check-docs.sh
bash scripts/check-ui.sh

echo "== [2/5] 25-scenario MCP evaluation (with machine-readable report) =="
bash scripts/mcp-eval.sh

echo "== [3/5] Project/block and media decoder fuzz harnesses =="
cargo test -p alp-core --test fuzz_project
cargo test -p alp-media --test fuzz_decode

echo "== [4/5] Playback soak proxy =="
cargo test -p alp-session --test long_horizon_soak
if (( SOAK_HOURS > 0 )); then
  if [[ "${SOAK_HOURS}" != "24" ]]; then
    echo "Only --hours 24 is supported by the ignored soak test; got ${SOAK_HOURS}" >&2
    exit 2
  fi
  echo "== [4b/5] Long-horizon 24-hour simulated soak =="
  cargo test -p alp-session --test long_horizon_soak -- --ignored
else
  echo "  (Skipping 24h soak; re-run with --soak --hours 24 to include it.)"
fi

echo "== [5/5] Workspace gates =="
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked

echo "Phase 18 acceptance checks passed"
