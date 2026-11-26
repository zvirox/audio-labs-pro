#!/usr/bin/env bash
# Audio Labs Pro — Automated MCP Evaluation Suite Harness
# Executes architecture validation, builds alp-mcp, and runs the legacy suites plus the 25 named Phase 18 scenarios.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

echo "== [1/3] Running workspace guards & architecture integrity checks =="
bash scripts/check-file-length.sh
bash scripts/check-deps.sh
bash scripts/check-docs.sh
bash scripts/check-ui.sh

echo ""
echo "== [2/3] Building alp-mcp server binary =="
cargo build --bin alp-mcp

MCP_BIN="${ROOT}/target/debug/alp-mcp"
if [[ ! -x "${MCP_BIN}" ]]; then
    echo "Error: target binary ${MCP_BIN} not found or not executable"
    exit 1
fi

echo ""
echo "== [3/3] Executing MCP Benchmark Suites + 25 Named Scenarios =="
python3 "${ROOT}/scripts/mcp_eval.py" "${MCP_BIN}" "${ROOT}/target/mcp_eval_results.json"

echo ""
echo "PASS: MCP evaluation completed with 100% pass rate (report: target/mcp_eval_results.json)."
