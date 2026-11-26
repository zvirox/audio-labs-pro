#!/usr/bin/env bash
# Phase 17 acceptance demo: MacroChain, VST3 discovery, safe X11 bridge, MCP tool.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

echo "== [1/5] Repository guards =="
bash scripts/check-file-length.sh
bash scripts/check-deps.sh
bash scripts/check-docs.sh
bash scripts/check-ui.sh

echo "== [2/5] Locked Linear UI gate =="
npm --prefix frontend run check:linear

echo "== [3/5] MacroChain batch engine =="
cargo test -p alp-engine macro_chain

echo "== [4/5] VST3 discovery, X11 bridge and macro MCP tool =="
cargo test -p alp-plugin vst3
cargo test -p alp-desktop plugin_window
cargo test -p alp-mcp

echo "== [5/5] Macro automation API =="
cargo test -p alp-api macro_ops

echo "Phase 17 acceptance checks passed"
