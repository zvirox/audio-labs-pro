# 0001 — License, platform and language scope (D1)

- Status: accepted
- Date: 2026-10-22
- Decider: project owner

## Context

D1 (open-source vs paid) decides which dependencies are usable: Slint and Qt are
GPL/commercial, LAME is LGPL, espeak-ng is GPL and several TTS weights are
non-commercial. The plan also locks Linux-only and English-only delivery.

## Decision

- The application is **Apache-2.0**. `LICENSE` at the repo root is the license.
- Platform: **Linux only**, Wayland and X11.
- Language: **English only** for UI and shipped models in v1.
- Dependencies must be Apache-2.0, MIT, BSD, ISC, Zlib, MPL-2.0 or similarly
  permissive. MPL-2.0 is allowed for unmodified crates (file-level copyleft).
- GPL/LGPL code and non-commercial weights are **not linked**. If a GPL tool is
  ever required, it runs as a separate process across a documented boundary and
  only after a written decision in a new ADR.

## Consequences

- espeak-ng cannot be a library dependency. Phase 0 proved Kokoro works with
  `piper-plus-g2p` (MIT, eSpeak-free) — see ADR 0010.
- MP3 encoding via LAME (LGPL) is deferred; v1 exports WAV/FLAC/OGG Vorbis
  (ADR 0005).
- Every new crate and model must be added to `THIRD_PARTY.md` with its license
  and source before use.
