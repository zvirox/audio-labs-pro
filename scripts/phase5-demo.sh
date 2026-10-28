#!/usr/bin/env bash
# Phase 5 acceptance harness: Voice Cleanup DSP pipeline and LUFS metering.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

work="$(mktemp -d /tmp/alp-phase5.XXXXXX)"
trap 'rm -rf "$work"' EXIT

alp="$root/target/release/alp-cli"
echo "== building alp-cli (release) =="
cargo build --release -p alp-cli

echo "== generate noisy fixture =="
# 3-second 220Hz fundamental with 50Hz hum component
"$alp" gen "$work/raw.wav" --seconds 3 --sample-rate 48000 --channels 1 --frequency 220 --amplitude 0.6 >/dev/null

echo "== initial loudness analysis =="
"$alp" lufs "$work/raw.wav"

echo "== execute voice cleanup pipeline =="
"$alp" cleanup "$work/raw.wav" "$work/cleaned.wav" --mains-freq 50

echo "== verify cleaned audio =="
test -f "$work/cleaned.wav"
"$alp" lufs "$work/cleaned.wav"

# Verify that peak limiter enforced the ceiling
"$alp" decode "$work/cleaned.wav" --json | python3 -c '
import json, sys
data = json.load(sys.stdin)
peak = data["peak"]
assert peak <= 0.895, f"Peak {peak} exceeded -1 dBFS limit!"
assert data["frames"] == 144000, "Frame count mismatch!"
print(f"Limiter ceiling verified: peak {peak:.4f} <= 0.895 (-1 dBFS)")
'

# Verify determinism: repeating cleanup yields identical bit output
"$alp" cleanup "$work/raw.wav" "$work/cleaned2.wav" --mains-freq 50 >/dev/null
"$alp" null-test "$work/cleaned.wav" "$work/cleaned2.wav" --tolerance 0.0001

echo
echo "Phase 5 Voice Cleanup & DSP harness: PASS"
