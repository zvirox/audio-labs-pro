# 0006 — Dictation model (D6)

- Status: accepted (runtime not yet measured)
- Date: 2026-10-22

## Context

Dictation needs English streaming ASR with a permissive license and a
CPU-friendly footprint.

## Decision

- Primary: **Moonshine tiny/base (English)** via ONNX Runtime — MIT
  (`onnx-community/moonshine-tiny-ONNX`, license verified through the HF API).
- Fallback: **Whisper `.en`** (MIT) only if Moonshine streaming quality fails on
  the test set in Phase 7.
- VAD: a permissively licensed VAD will be chosen in Phase 7 (candidates:
  Silero VAD MIT, WebRTC VAD BSD). No VAD decision is load-bearing yet.

## Evidence / gaps

- License and model availability verified (HF API, MIT).
- Kokoro through the same `ort` stack runs at RTF 0.26–0.36, so the inference
  path is proven; **Moonshine itself was not executed in Phase 0** because the
  same session ran out of budget. This is the one acceptance gap of Phase 0 and
  the first task of the Phase 7 spike.

## Consequences

- Dictation is in-app only until Wayland typing-injection is proven; no
  system-wide injection promise in v1.
- Models are download-tier, checksum-verified, stored in XDG data dirs.
