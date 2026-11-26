#!/usr/bin/env bash
# Phase 15 acceptance demo: ACX QA and native M4B export.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

echo "== [1/4] Repository guards =="
bash scripts/check-file-length.sh
bash scripts/check-deps.sh
bash scripts/check-docs.sh
bash scripts/check-ui.sh

echo "== [2/4] Locked Linear UI gate =="
npm --prefix frontend run check:linear

echo "== [3/4] ACX QA tests =="
cargo test -p alp-dsp true_peak
cargo test -p alp-api acx_qa

echo "== [4/4] Native M4B export test =="
cargo test -p alp-media m4b

echo "Phase 15 acceptance checks passed"
