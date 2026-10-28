#!/usr/bin/env bash
# Phase 6 acceptance harness: model manager, alp-infer worker, TTS, and script mode.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

tmp="$(mktemp -d /tmp/alp-phase6.XXXXXX)"
trap 'rm -rf "$tmp"' EXIT

echo "== building alp-cli and alp-infer (release) =="
cargo build --release -p alp-cli -p alp-infer

CLI="$root/target/release/alp-cli"
INFER="$root/target/release/alp-infer"
SOCKET="$tmp/infer.sock"

echo "== model catalogue inspection =="
"$CLI" model list
"$CLI" model accept-license kokoro-82m

echo "== direct speech synthesis via CLI =="
"$CLI" tts "Welcome to Audio Labs Pro Phase 6." "$tmp/welcome.wav" --voice aura --speed 1.0

# Verify file exists and has non-zero size
if [ ! -s "$tmp/welcome.wav" ]; then
    echo "FAIL: welcome.wav was not created or is empty"
    exit 1
fi

echo "== launch out-of-process alp-infer daemon =="
"$INFER" --socket "$SOCKET" &
INFER_PID=$!
trap 'kill "$INFER_PID" 2>/dev/null || true; rm -rf "$tmp"' EXIT

# Wait briefly for socket to become available
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

echo "== synthesize speech via alp-infer worker =="
XDG_RUNTIME_DIR="$tmp" "$CLI" tts "Synthesizing through the isolated out-of-process inference worker." "$tmp/socket_synth.wav" --voice atlas --speed 1.0

if [ ! -s "$tmp/socket_synth.wav" ]; then
    echo "FAIL: socket_synth.wav was not created"
    exit 1
fi

echo "== multi-line script rendering and stitching =="
cat <<JSON > "$tmp/script.json"
{
  "title": "Phase 6 Narration",
  "default_voice": "aura",
  "lines": [
    {
      "id": "intro",
      "text": "Chapter one: The new audio engine.",
      "voice_id": "aura",
      "speed": 1.0,
      "status": "Pending"
    },
    {
      "id": "body",
      "text": "Everything was built in clean, memory-safe Rust with zero audio dropouts.",
      "voice_id": "echo",
      "speed": 1.0,
      "status": "Pending"
    },
    {
      "id": "outro",
      "text": "This concludes our demonstration.",
      "voice_id": "sol",
      "speed": 1.0,
      "status": "Pending"
    }
  ]
}
JSON

"$CLI" script render "$tmp/script.json" "$tmp/stitched_script.wav" --stitch

if [ ! -s "$tmp/stitched_script.wav" ]; then
    echo "FAIL: stitched_script.wav was not created"
    exit 1
fi

echo "== script stem export =="
mkdir -p "$tmp/stems"
"$CLI" script render "$tmp/script.json" "$tmp/stems"

STEM_COUNT=$(ls -1 "$tmp/stems"/*.wav 2>/dev/null | wc -l)
if [ "$STEM_COUNT" -ne 3 ]; then
    echo "FAIL: expected 3 stems, found $STEM_COUNT"
    exit 1
fi
echo "verified 3 exported stems"

echo "== determinism null-test between successive runs =="
"$CLI" tts "Testing deterministic acoustic synthesis." "$tmp/run1.wav" --voice aura
"$CLI" tts "Testing deterministic acoustic synthesis." "$tmp/run2.wav" --voice aura

"$CLI" null-test "$tmp/run1.wav" "$tmp/run2.wav" --tolerance 0.0001

echo "Phase 6 Model Manager & TTS harness: PASS"
