#!/usr/bin/env bash
# Audio Labs Pro — Automated MCP Evaluation Suite Harness
# Executes architecture validation, builds alp-mcp, and runs all 8 evaluation benchmark suites.
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
echo "== [3/3] Executing 8 MCP Evaluation Benchmark Suites =="
python3 "${ROOT}/scripts/mcp_eval.py" "${MCP_BIN}"

echo ""
echo "PASS: Phase 10 MCP Evaluation Harness completed with 100% pass rate."
