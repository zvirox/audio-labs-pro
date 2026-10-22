# 0010 — Text-to-phoneme without espeak-ng

- Status: accepted
- Date: 2026-10-22
- Spike: `spikes/model-spike`

## Context

Kokoro and Kitten take phoneme IDs, not text. The usual front-end is
espeak-ng, which is **GPL-3.0** and therefore cannot be linked into an
Apache-2.0 app (ADR 0001). Running espeak-ng as a helper process would work
legally but adds a shipped GPL binary, a process hop per utterance and a
platform dependency.

## Options

| Option | License | Notes |
|---|---|---|
| espeak-ng via FFI | GPL-3.0 | rejected: copyleft infection |
| espeak-ng as separate process | GPL-3.0 | legal but heavy; needs packaging review |
| `piper-plus-g2p` 0.5.2 | MIT | pure Rust, IPA output, English via CMUdict |
| `floravox-g2p` 0.8.8 | Apache-2.0 OR MIT | FST lexicon + OOV fallback, less tested here |
| `voirs-g2p` 0.1.0-rc.1 | Apache-2.0 | pre-release |

## Decision

Use **`piper-plus-g2p`** (English feature only) with the CMUdict data file
shipped in assets. No GPL code enters the build.

## Evidence

End-to-end run on this machine:

```
g2p: "Hello from Audio Labs Pro." -> "həlˈoʊ fɹʌm ˈɑdiːˌoʊ lˈæb pɹˈoʊ."
tokens=34 skipped=""
Kokoro-82M inference: 589–822 ms for 2.25 s audio (RTF 0.26–0.36)
```

All phonemes mapped into Kokoro's 115-symbol vocabulary; output is non-silent
(peak −8.9 dBFS, RMS −23.6 dBFS, zero-crossing rate 0.217, flat factor 0.0 —
speech-like, not noise).

## Consequences

- Text normalization (numbers, currency, abbreviations) is still our job before
  G2P; it becomes a `alp-voice` module with tests.
- CMUdict must be recorded in `THIRD_PARTY.md` with its BSD-style license.
- A pronunciation dictionary (word → phonemes) layers on top of this front-end.
