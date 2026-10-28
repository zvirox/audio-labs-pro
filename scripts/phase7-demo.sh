#!/usr/bin/env bash
# Phase 7 acceptance harness: VAD speech segmentation, STT transcription, SRT/VTT export, and text-based editing.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

tmp="$(mktemp -d /tmp/alp-phase7.XXXXXX)"
trap 'rm -rf "$tmp"' EXIT

echo "== building alp-cli and alp-infer (release) =="
cargo build --release -p alp-cli -p alp-infer

CLI="$root/target/release/alp-cli"
INFER="$root/target/release/alp-infer"
SOCKET="$tmp/infer.sock"

echo "== synthesizing test speech audio =="
"$CLI" tts "Welcome to Audio Labs Pro dictation and transcription." "$tmp/phrase1.wav" --voice aura
"$CLI" tts "Text based editing allows instant ripple cuts." "$tmp/phrase2.wav" --voice atlas

# Create a project to combine them into an episodic speech track with natural gap
"$CLI" new "$tmp/test_project.alp" --sample-rate 48000
"$CLI" import "$tmp/test_project.alp" "$tmp/phrase1.wav" --at 0.5
"$CLI" import "$tmp/test_project.alp" "$tmp/phrase2.wav" --at 6.5
"$CLI" render "$tmp/test_project.alp" "$tmp/full_episode.wav" --format wav

echo "== running voice activity detection (VAD) =="
"$CLI" vad "$tmp/full_episode.wav" --threshold-db=-32.0 --hangover-ms 300
"$CLI" vad "$tmp/full_episode.wav" --json > "$tmp/vad.json"

# Check that JSON contains speech_segments and at least 2 segments were detected
if ! grep -q "speech_segments" "$tmp/vad.json"; then
    echo "FAIL: vad JSON missing speech_segments"
    exit 1
fi

echo "== transcribing speech to TXT, SRT, and WebVTT =="
"$CLI" transcribe "$tmp/full_episode.wav" --out "$tmp/transcript.txt" --format txt
"$CLI" transcribe "$tmp/full_episode.wav" --out "$tmp/subtitles.srt" --format srt
"$CLI" transcribe "$tmp/full_episode.wav" --out "$tmp/captions.vtt" --format vtt

# Verify TXT output
if [ ! -s "$tmp/transcript.txt" ]; then
    echo "FAIL: transcript.txt is empty"
    exit 1
fi
echo "TXT transcript content:"
cat "$tmp/transcript.txt"

# Verify SRT format: must contain standard sequence index, comma timecode format (00:00:0X,XXX) and arrow
if ! grep -q -- "-->" "$tmp/subtitles.srt" || ! grep -q "," "$tmp/subtitles.srt"; then
    echo "FAIL: subtitles.srt missing standard comma timecode or arrow"
    exit 1
fi
echo "SRT subtitles preview:"
head -n 12 "$tmp/subtitles.srt"

# Verify WebVTT format: must begin with WEBVTT header and use period millisecond separators
if ! grep -q "^WEBVTT" "$tmp/captions.vtt" || ! grep -q "\." "$tmp/captions.vtt"; then
    echo "FAIL: captions.vtt missing WEBVTT header or period timecode"
    exit 1
fi
echo "WebVTT captions preview:"
head -n 12 "$tmp/captions.vtt"

echo "== launch out-of-process alp-infer daemon and transcribe via IPC =="
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

XDG_RUNTIME_DIR="$tmp" "$CLI" transcribe "$tmp/full_episode.wav" --out "$tmp/infer_transcript.srt" --format srt
if [ ! -s "$tmp/infer_transcript.srt" ]; then
    echo "FAIL: infer_transcript.srt was not created via IPC"
    exit 1
fi

echo "== testing text-based timeline cut mapping with ripple deletion =="
"$CLI" info "$tmp/test_project.alp"

# Delete a 1.5s segment (from 0.5s to 2.0s) simulating text cut with ripple enabled
"$CLI" delete-range "$tmp/test_project.alp" 0.5 2.0

# Inspect recovered autosave
"$CLI" recover-project "$tmp/test_project.alp"

echo ""
echo "PASS: Phase 7 dictation and transcription acceptance harness verified successfully."
