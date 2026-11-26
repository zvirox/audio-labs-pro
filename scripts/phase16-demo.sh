#!/usr/bin/env bash
# Phase 16 acceptance demo: rate fitting, video remux, model-gated Demucs, Dubbing UI.
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

echo "== [3/5] WSOLA and FitPlanner =="
cargo test -p alp-media time_stretch
cargo test -p alp-voice rate_fitting

echo "== [4/5] Video remux and Demucs runner =="
cargo test -p alp-media video_sync
cargo test -p alp-infer demucs

echo "== [5/5] Dubbing IPC bridge =="
cargo test -p alp-session dubbing

echo "Phase 16 acceptance checks passed"
