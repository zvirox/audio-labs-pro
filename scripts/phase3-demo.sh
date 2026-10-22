#!/usr/bin/env bash
# Phase 3 acceptance harness.
#
# Headless by default:
#   1. create a project and import 44.1 kHz and 48 kHz sources;
#   2. write an autosave, make another edit, then recover the autosave;
#   3. render a float reference, export 24-bit WAV and FLAC;
#   4. null-test both exports against the reference.
#
# Optional hardware checks:
#   --record   run the forced-kill capture recovery test
#   --soak     record a 30-minute take and report dropout health
#   --minutes N  soak length in minutes (default 30)
#   --keep     keep the temporary directory
#
# The workspace is created under $TMPDIR (default /tmp). A 30-minute stereo 48 kHz
# float32 capture needs about 1.5 GB of free space for the spool plus the output
# WAV. If $TMPDIR is small (for example an overlay with 2 GB), rerun with
#   TMPDIR=/path/big/disk bash scripts/phase3-demo.sh ...
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

record=0
soak=0
minutes=30
keep=0
while (($#)); do
  case "$1" in
    --record) record=1 ;;
    --soak) soak=1 ;;
    --minutes)
      shift
      minutes="${1:?--minutes needs a value}"
      ;;
    --keep) keep=1 ;;
    -h|--help)
      sed -n '2,19p' "$0"
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      exit 2
      ;;
  esac
  shift
done

echo "== building alp-cli (release) =="
cargo build --release -p alp-cli
alp="$root/target/release/alp-cli"

work="$(mktemp -d "${TMPDIR:-/tmp}/alp-phase3.XXXXXX")"
cleanup() {
  if (( keep == 0 )); then
    rm -rf "$work"
  else
    echo "artifacts kept in $work"
  fi
}
trap cleanup EXIT

project="$work/edit.alp"
"$alp" new "$project" --name "Phase 3 Edit"
"$alp" gen "$work/tone-48.wav" --seconds 3 --sample-rate 48000 --channels 1 \
  --frequency 220 --amplitude 0.2
"$alp" gen "$work/tone-44.wav" --seconds 2 --sample-rate 44100 --channels 1 \
  --frequency 330 --amplitude 0.15

echo
echo "== 1. import and format conversion =="
"$alp" import "$project" "$work/tone-48.wav" --at 0 --name "Tone 48"
"$alp" import "$project" "$work/tone-44.wav" --at 3 --name "Tone 44"
"$alp" info "$project" --json >"$work/info.json"
python3 - "$work/info.json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    info = json.load(handle)
if info["clips"] != 2:
    raise SystemExit(f"FAIL expected 2 clips, got {info['clips']}")
if any(source["sample_rate"] != 48000 for source in info["sources"]):
    raise SystemExit("FAIL import did not convert every source to 48 kHz")
if abs(info["duration_secs"] - 5.0) > 0.02:
    raise SystemExit(f"FAIL expected 5.0s, got {info['duration_secs']:.3f}s")
print(f"import: {info['clips']} clips · {info['duration_secs']:.3f}s · all 48 kHz")
PY

echo
echo "== 2. autosave recovery =="
"$alp" import "$project" "$work/tone-48.wav" --at 8 --name "Longer session"
"$alp" autosave "$project"
"$alp" delete-range "$project" 1 2
"$alp" info "$project" --json >"$work/clean.json"
"$alp" recover-project "$project" --json >"$work/recovered.json"
"$alp" info "$project" --json >"$work/after-recover.json"
python3 - "$work/clean.json" "$work/after-recover.json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    clean = json.load(handle)
with open(sys.argv[2], encoding="utf-8") as handle:
    after = json.load(handle)
if clean["clips"] != 3:
    raise SystemExit(f"FAIL clean file changed unexpectedly: {clean['clips']} clips")
if after["clips"] != 4:
    raise SystemExit(f"FAIL recovery restored {after['clips']} clips, expected 4")
print("autosave: clean=3 clips · recovered edited=4 clips · recovery verified")
PY

echo
echo "== 3. WAV/FLAC export and null test =="
reference="$work/reference.wav"
wav="$work/mix.wav"
flac="$work/mix.flac"
"$alp" render "$project" "$reference" --bit-depth float32 --dither none
"$alp" render "$project" "$wav" --bit-depth int24 --dither triangular \
  --sample-rate 48000 --title "Phase 3 mix" --artist "Audio Labs Pro" \
  --album "Demo session"
"$alp" render "$project" "$flac" --format flac --bit-depth int24 \
  --dither triangular --sample-rate 48000 --title "Phase 3 mix" \
  --artist "Audio Labs Pro"
"$alp" null-test "$reference" "$wav" --tolerance 0.001 --json >"$work/null-wav.json"
"$alp" null-test "$reference" "$flac" --tolerance 0.001 --json >"$work/null-flac.json"
"$alp" decode "$flac" --json >"$work/flac.json"
python3 - "$work/null-wav.json" "$work/null-flac.json" "$work/flac.json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as wav:
    wav_report = json.load(wav)
with open(sys.argv[2], encoding="utf-8") as flac:
    flac_report = json.load(flac)
with open(sys.argv[3], encoding="utf-8") as decoded:
    flac_info = json.load(decoded)
if not wav_report["passed"]:
    raise SystemExit(f"FAIL WAV null test: {wav_report}")
if not flac_report["passed"]:
    raise SystemExit(f"FAIL FLAC null test: {flac_report}")
if flac_info["codec"].lower() != "flac":
    raise SystemExit(f"FAIL FLAC decode reported codec {flac_info['codec']}")
print(
    f"null test: WAV {wav_report['max_abs_diff']:.8f} · "
    f"FLAC {flac_report['max_abs_diff']:.8f} · threshold 0.001"
)
PY

if (( record == 1 || soak == 1 )); then
  echo
  echo "== 4. forced-kill capture recovery =="
  crash_capture="$work/crash.wav"
  "$alp" record "$crash_capture" --seconds 1800 --chunk-seconds 2 \
    >"$work/crash-record.log" 2>&1 &
  recorder=$!
  sleep 8
  kill -9 "$recorder" 2>/dev/null || true
  wait "$recorder" 2>/dev/null || true
  "$alp" recover "$crash_capture" --json >"$work/crash-recover.json"
  "$alp" decode "$crash_capture" --json >"$work/crash-decode.json"
  python3 - "$work/crash-recover.json" "$work/crash-decode.json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    report = json.load(handle)
with open(sys.argv[2], encoding="utf-8") as handle:
    decode = json.load(handle)
if not report["recovered"]:
    raise SystemExit("FAIL recovery report did not mark the take as recovered")
if decode["frames"] < 48_000:
    raise SystemExit(f"FAIL recovered only {decode['frames']} frames")
print(f"capture recovery: {decode['frames']} frames · recovered=true")
PY
fi

if (( soak == 1 )); then
  echo
  echo "== 5. ${minutes}-minute dropout soak =="
  soak_capture="$work/soak.wav"
  "$alp" record "$soak_capture" --seconds "$((minutes * 60))" \
    --chunk-seconds 10 | tee "$work/soak-record.log"
  "$alp" decode "$soak_capture" --json >"$work/soak.json"
  python3 - "$work/soak.json" "$work/soak-record.log" "$minutes" <<'PY'
import json
import sys

expected = int(float(sys.argv[3]) * 60 * 48_000)
with open(sys.argv[1], encoding="utf-8") as handle:
    info = json.load(handle)
if info["frames"] < expected - 48_000:
    raise SystemExit(f"FAIL soak captured {info['frames']} frames, expected {expected}")
with open(sys.argv[2], encoding="utf-8") as handle:
    log = handle.read()
if "stream error" in log:
    raise SystemExit("FAIL soak recorded a backend stream error")
dropped = 0
for line in log.splitlines():
    if line.startswith("dropped:"):
        dropped = int(line.split()[1])
if dropped:
    raise SystemExit(f"FAIL soak dropped {dropped} samples")
print(f"soak: {info['frames']} frames · {info['duration_secs']:.3f}s · dropped 0 samples")
PY
fi

echo
echo "Phase 3 demo: PASS"
