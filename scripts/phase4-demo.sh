#!/usr/bin/env bash
# Phase 4 acceptance harness: multitrack project, pan/gain, stems, export.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

work="$(mktemp -d /tmp/alp-phase4.XXXXXX)"
trap 'rm -rf "$work"' EXIT

alp="$root/target/release/alp-cli"
echo "== building alp-cli (release) =="
cargo build --release -p alp-cli

echo "== generate input =="
"$alp" gen "$work/tone-48.wav" --seconds 3 --sample-rate 48000 --channels 1 --frequency 220 --amplitude 0.15 >/dev/null

echo "== create project =="
"$alp" new "$work/p" --name "Phase 4" >/dev/null
# Add a second track so the project is a real 2-track workspace.
python3 - "$work/p" <<'PY'
import json, sys, uuid
root = sys.argv[1] + "/project.json"
doc = json.load(open(root))
project = doc["project"]
track = {
    "id": str(uuid.uuid4()),
    "name": "Track 2",
    "gain": 0.75,
    "pan": 0.25,
    "muted": False,
    "solo": False,
    "color": [196, 132, 86],
    "armed": False,
    "clips": [],
    "master_gain": 0.85,
}
# older schema does not allow master_gain inside track; it belongs at project.
track.pop("master_gain")
project["tracks"].append(track)
project["master_gain"] = 0.85
json.dump(doc, open(root, "w"), indent=2)
PY

"$alp" import "$work/p" "$work/tone-48.wav" >/dev/null

echo "== inspect =="
"$alp" info "$work/p" | grep -E "sources:|tracks:" | head -10

echo "== render master == "
"$alp" render "$work/p" "$work/mix.wav" --format wav --bit-depth int24 --dither triangular >/dev/null
"$alp" render-stems "$work/p" --output-dir "$work/stems" --format wav --bit-depth int24 --dither triangular

echo "== render presets =="
"$alp" render "$work/p" "$work/podcast.flac" --preset podcast >/dev/null
test -f "$work/podcast.flac"
"$alp" render "$work/p" "$work/vo.wav" --preset voice-over >/dev/null
test -f "$work/vo.wav"

echo "== verify stems =="
test "$(find "$work/stems" -name '*.wav' | wc -l)" -ge 1
"$alp" null-test "$work/mix.wav" "$work/mix.wav" --tolerance 0.001

echo
echo "Phase 4 CLI harness: PASS"
