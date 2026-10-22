# 0005 — Export formats at v1 (D5)

- Status: accepted
- Date: 2026-10-22
- Decider: project owner (WAV, FLAC, OGG Vorbis)

## Context

The plan proposed WAV, FLAC, MP3 and OGG Vorbis at v1. The license pass found:

| Codec | Candidate | License | Verdict |
|---|---|---|---|
| WAV | `hound` | MIT | ship |
| FLAC | `flacenc` | Apache-2.0 | ship |
| OGG Vorbis | `vorbis_rs` | BSD-3-Clause | ship |
| MP3 | `mp3lame-encoder` / LAME | LGPL-3.0 | not linkable under Apache-2.0 plan without extra obligations |
| Opus | `opus` bindings | BSD-3-Clause | post-1.0 |

## Decision

v1 exports **WAV (PCM 16/24/32-bit float), FLAC and OGG Vorbis**, with sample
rate, bit depth, dither and metadata tags. MP3 is deferred until an LGPL review
or a permissively licensed encoder is available; the export UI will show a
disabled "MP3 (not available)" entry with the reason.

## Consequences

- Export presets (podcast, audiobook, voice-over) are built on the three codecs.
- No LGPL binary is bundled in the AppImage/Flatpak.
- Reintroducing MP3 requires a new ADR covering dynamic linking, source
  availability and Flatpak relinking rules.
