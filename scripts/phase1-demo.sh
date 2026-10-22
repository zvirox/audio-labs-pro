#!/usr/bin/env bash
# Phase 1 acceptance harness.
#
# Headless by default:
#   1. generate a one-hour WAV and build/reopen its peak cache;
#   2. create an eight-clip project and render it;
#   3. inspect the render.
#
# Optional:
#   --play    play ten seconds on the default output device
#   --soak    play for ten minutes and report callback/stream health
#   --record  record five seconds from the default input device
#   --keep    keep the temporary directory
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

play=0
soak=0
record=0
keep=0
for argument in "$@"; do
  case "$argument" in
    --play) play=1 ;;
    --soak) soak=1 ;;
    --record) record=1 ;;
    --keep) keep=1 ;;
    -h|--help)
      sed -n '2,16p' "$0"
      exit 0
      ;;
    *)
      echo "unknown option: $argument" >&2
      exit 2
      ;;
  esac
done

echo "== building alp-cli (release) =="
cargo build --release -p alp-cli
alp="$root/target/release/alp-cli"

work="$(mktemp -d "${TMPDIR:-/tmp}/alp-phase1.XXXXXX")"
cleanup() {
  if (( keep == 0 )); then
    rm -rf "$work"
  else
    echo "artifacts kept in $work"
  fi
}
trap cleanup EXIT

echo
echo "== 1. one-hour WAV: peak cache miss then hit =="
tone="$work/hour.wav"
start=$(date +%s.%N)
"$alp" gen "$tone" --seconds 3600 --sample-rate 48000 --channels 1 --frequency 180
end=$(date +%s.%N)
echo "generate: $(python3 -c "print(f'{$end-$start:.2f}s')")"

"$alp" peaks "$tone" --json >"$work/peaks-first.json"
"$alp" peaks "$tone" --json >"$work/peaks-second.json"
python3 - "$work/peaks-first.json" "$work/peaks-second.json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    first = json.load(handle)
with open(sys.argv[2], encoding="utf-8") as handle:
    second = json.load(handle)

if first["status"] != "miss":
    raise SystemExit(f"FAIL first peaks run was {first['status']}, expected miss")
if second["status"] != "hit":
    raise SystemExit(f"FAIL second peaks run was {second['status']}, expected hit")
if second["elapsed_secs"] > 0.25:
    raise SystemExit(
        f"FAIL cache hit took {second['elapsed_secs']:.3f}s, expected near-instant"
    )
print(
    f"peaks: first={first['status']} {first['elapsed_secs']:.3f}s · "
    f"second={second['status']} {second['elapsed_secs']:.3f}s · "
    f"{second['frames']} frames · {len(second['levels'])} levels"
)
PY

echo
echo "== 2. eight simultaneous clips =="
project="$work/eight-clip.alp"
clip="$work/clip.wav"
clip_seconds=60
clip_channels=2
clip_amplitude=0.05
if (( soak == 1 )); then
  # A real ten-minute soak needs ten minutes of material; mono keeps RAM and
  # disk bounded while still mixing eight independent sources.
  clip_seconds=600
  clip_channels=1
  clip_amplitude=0.01
fi
"$alp" gen "$clip" --seconds "$clip_seconds" --sample-rate 48000 \
  --channels "$clip_channels" --frequency 220 --amplitude "$clip_amplitude"
"$alp" new "$project" --name "Phase 1 Eight Clip"
for _ in 1 2 3 4 5 6 7 8; do
  "$alp" import "$project" "$clip" --at 0 --name "Clip"
done

"$alp" info "$project" --json >"$work/info.json"
python3 - "$work/info.json" "$clip_seconds" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    info = json.load(handle)
expected = float(sys.argv[2])
if info["tracks"][0]["clips"] != 8:
    raise SystemExit(f"FAIL expected 8 clips, got {info['tracks'][0]['clips']}")
if abs(info["duration_secs"] - expected) > 0.01:
    raise SystemExit(
        f"FAIL expected {expected:.2f}s, got {info['duration_secs']:.2f}s"
    )
print(f"project: {info['clips']} clips · {info['duration_secs']:.2f}s")
PY

if (( soak == 0 )); then
  mix="$work/eight-clip.wav"
  start=$(date +%s.%N)
  "$alp" render "$project" "$mix" --bit-depth float32
  end=$(date +%s.%N)
  render_secs=$(python3 -c "print(f'{$end-$start:.2f}')")

  "$alp" decode "$mix" --json >"$work/render.json"
  python3 - "$work/render.json" "$clip_seconds" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    render = json.load(handle)
expected = int(float(sys.argv[2]) * 48_000)
if render["frames"] != expected:
    raise SystemExit(f"FAIL expected {expected} frames, got {render['frames']}")
if render["peak"] <= 0.1:
    raise SystemExit(f"FAIL render looks silent (peak {render['peak']})")
print(
    f"render: {render['frames']} frames · peak {render['peak']:.3f} · "
    f"rms {render['rms']:.3f}"
)
PY
  echo "render wall time: ${render_secs}s"
fi

if (( play == 1 || soak == 1 )); then
  seconds=10
  label="10 s playback"
  if (( soak == 1 )); then
    seconds=600
    label="10 min soak"
  fi
  echo
  echo "== 3. ${label} on the default output device =="
  "$alp" play "$project" --seconds "$seconds" | tee "$work/play.txt"
  if grep -q "stream_error=true" "$work/play.txt"; then
    echo "FAIL playback reported a stream error" >&2
    exit 1
  fi
  python3 - "$work/play.txt" <<'PY'
import re
import sys

text = open(sys.argv[1], encoding="utf-8").read()
callbacks = int(re.search(r"callbacks=(\d+)", text).group(1))
frames = int(re.search(r"frames=(\d+)", text).group(1))
if callbacks == 0 or frames == 0:
    raise SystemExit("FAIL playback produced no callbacks or frames")
print(f"playback: {callbacks} callbacks · {frames} frames · no stream error")
PY
fi

if (( record == 1 )); then
  echo
  echo "== 4. five-second capture =="
  capture="$work/capture.wav"
  "$alp" record "$capture" --seconds 5
  "$alp" decode "$capture" --json >"$work/capture.json"
  python3 - "$work/capture.json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    capture = json.load(handle)
if capture["frames"] == 0:
    raise SystemExit("FAIL capture wrote no frames")
print(f"capture: {capture['frames']} frames · {capture['channels']} ch")
PY
fi

echo
echo "Phase 1 demo: PASS"
