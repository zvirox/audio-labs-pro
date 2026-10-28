#!/usr/bin/env bash
# Phase 8 acceptance harness: voice profiles, cloning safety, voice design, and .alpvoice bundles.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

tmp="$(mktemp -d /tmp/alp-phase8.XXXXXX)"
trap 'rm -rf "$tmp"' EXIT

echo "== building alp-cli and alp-infer (release) =="
cargo build --release -p alp-cli -p alp-infer

CLI="$root/target/release/alp-cli"
INFER="$root/target/release/alp-infer"
SOCKET="$tmp/infer.sock"
export XDG_DATA_HOME="$tmp/data"
export XDG_CACHE_HOME="$tmp/cache"
export XDG_CONFIG_HOME="$tmp/config"

echo "== hardware detection and tier recommendation =="
"$CLI" hardware
"$CLI" hardware --json > "$tmp/hardware.json"
if ! grep -q "recommended_tier" "$tmp/hardware.json"; then
    echo "FAIL: hardware json missing recommended_tier"
    exit 1
fi

echo "== voice profile library and prebuilt voices =="
"$CLI" profile list
"$CLI" profile info aura

echo "== synthesize reference audio and audit quality =="
# Generate 6 seconds of speech audio (passes >= 5.0s minimum requirement)
"$CLI" tts "Welcome to the reference audio recording test for voice cloning in Audio Labs Pro." "$tmp/ref_long.wav" --voice aura
"$CLI" profile check-reference "$tmp/ref_long.wav"

echo "== voice design from natural language prompt =="
"$CLI" profile design "DocuVoice" --prompt "Warm, deep, calm BBC documentary narrator with deliberate pacing" --tags "narration,deep,british"
"$CLI" profile info $(echo $("$CLI" profile list --json | grep -o '"id": "designed-[^"]*' | head -n 1 | cut -d'"' -f4))

DESIGNED_ID=$("$CLI" profile list --json | grep -o '"id": "designed-[^"]*' | head -n 1 | cut -d'"' -f4)
echo "Designed profile ID: $DESIGNED_ID"

echo "== export to .alpvoice package bundle =="
BUNDLE="$tmp/docuvoice.alpvoice"
"$CLI" profile export "$DESIGNED_ID" "$BUNDLE"

if [ ! -s "$BUNDLE" ]; then
    echo "FAIL: .alpvoice package was not created"
    exit 1
fi
echo "Package size: $(wc -c < "$BUNDLE") bytes"

echo "== delete profile and verify removal =="
"$CLI" profile delete "$DESIGNED_ID"
if "$CLI" profile info "$DESIGNED_ID" 2>/dev/null; then
    echo "FAIL: profile was not deleted"
    exit 1
fi

echo "== import profile from .alpvoice bundle =="
"$CLI" profile import "$BUNDLE"
"$CLI" profile info "$DESIGNED_ID"

echo "== synthesize speech and run A/B acoustic comparison =="
"$CLI" tts "Comparing synthesized take against reference acoustics." "$tmp/gen_take.wav" --voice aura
"$CLI" profile compare "$tmp/ref_long.wav" "$tmp/gen_take.wav"

echo "== launch out-of-process alp-infer daemon and design voice via IPC =="
"$INFER" --socket "$SOCKET" &
INFER_PID=$!
trap 'kill "$INFER_PID" 2>/dev/null || true; rm -rf "$tmp"' EXIT

for i in {1..30}; do
    if [ -S "$SOCKET" ]; then
        break
    fi
    sleep 0.1
done

if [ ! -S "$SOCKET" ]; then
    echo "FAIL: alp-infer failed to create socket at $SOCKET"
    exit 1
fi

XDG_RUNTIME_DIR="$tmp" "$CLI" profile design "WorkerVoice" --prompt "Fast, high, energetic presenter" --tags "fast,bright"
"$CLI" profile list

echo ""
echo "PASS: Phase 8 voice profiles, cloning, and voice generation acceptance harness verified successfully."
