#!/usr/bin/env bash
# Phase 0 demo: reproduce every spike result in one run.
# Safe to run headless except the UI step, which needs a Wayland/X11 session.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root/spikes"

echo "== building spikes =="
cargo build --release --workspace

echo
echo "== blur protocol probe =="
./target/release/wayland-probe

echo
echo "== audio devices =="
./target/release/audio-spike devices

echo
echo "== playback: 5 s, ring buffer, no callback gaps expected =="
./target/release/audio-spike play 5

echo
echo "== capture: 3 s to /tmp/alp-demo-record.wav (skipped without an input device) =="
./target/release/audio-spike record /tmp/alp-demo-record.wav 3 || \
  echo "capture unavailable on this machine"

echo
echo "== 10-minute WAV: decode + peak pyramid + cache reload =="
./target/release/audio-spike gen /tmp/alp-demo.wav 600
./target/release/audio-spike peaks /tmp/alp-demo.wav /tmp/alp-demo.peaks

if [ -f "$HOME/.local/share/alp/models/kokoro/model.onnx" ]; then
  echo
  echo "== Kokoro TTS through ONNX Runtime (MIT G2P) =="
  cmudict="$(ls -d "$HOME"/.cargo/registry/src/*/piper-plus-g2p-*/data/cmudict_data.json 2>/dev/null | head -1)"
  CMUDICT_PATH="${cmudict:-}" ALP_SPIKE_G2P=1 ./target/release/model-spike | tail -8
else
  echo
  echo "== Kokoro spike skipped: model files not present =="
fi

if [ -n "${WAYLAND_DISPLAY:-}${DISPLAY:-}" ]; then
  echo
  echo "== iced glass + 1-hour waveform, 10 s measurement window =="
  ALP_SPIKE_AUTOCLOSE_SECS=10 ALP_SPIKE_METRICS=1 \
    ALP_SPIKE_START_TAB=waveform ALP_SPIKE_AUTOSCROLL=1 \
    ./target/release/ui-iced-spike
else
  echo
  echo "== UI spike skipped: no display =="
fi

echo
echo "Phase 0 demo complete. Numbers should match docs/spikes/PHASE0-RESULTS.md"
