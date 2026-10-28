# Third-party inventory

Rule: nothing is added to the build or shipped as an asset until it appears here
with a version and a license that is compatible with Apache-2.0 (see
`docs/adr/0001-license-platform-scope.md`).

Status legend: **P0** used by a Phase 0 spike, **P1** used by the Phase 1
workspace, **P2** used by the Phase 2 shell, **P3** used by Phase 3 recording and
editing, **P4-P8** used by multitrack, DSP, TTS, dictation, and voice profiles,
**P9** packaged for v1.0, **planned** approved but not yet wired into the app.

## Rust crates

| Crate | Version | License | Status | Purpose |
|---|---|---|---|---|
| iced | 0.14.0 | MIT | P2 | UI toolkit, pinned exactly; wgpu, canvas, advanced |
| lilt | 0.8.2 | MIT | P2 | time-based panel transitions |
| x11rb | 0.13.2 | MIT OR Apache-2.0 | P2 | KWin X11 blur property; safe Rust |
| cpal | 0.18.2 | Apache-2.0 | P1 | audio I/O |
| rtrb | 0.4.0 | MIT OR Apache-2.0 | P1 | lock-free SPSC ring buffer |
| symphonia | 0.6.1 | MPL-2.0 | P1 | decoding (unmodified; file-level copyleft only) |
| hound | 3.5.1 | Apache-2.0 | P1 | WAV read/write |
| rubato | 5.0.1 | MIT OR Apache-2.0 | P1 | offline FFT resampling |
| ort | 2.0.0-rc.15 | MIT OR Apache-2.0 | P0 | ONNX Runtime bindings; Phase 6 |
| ONNX Runtime (binary) | 1.31 | MIT | P0 | downloaded by `ort` |
| piper-plus-g2p | 0.5.2 | MIT | P0 | English G2P (eSpeak-free) |
| serde / serde_json | 1.0 | MIT OR Apache-2.0 | P1 | serialization |
| thiserror | 2.0 | MIT OR Apache-2.0 | P1 | typed errors |
| clap | 4.6 | MIT OR Apache-2.0 | P1 | CLI grammar |
| anyhow | 1.0 | MIT OR Apache-2.0 | P1 | CLI error context |
| dirs | 7.0 | MIT OR Apache-2.0 | P1 | XDG base directories |
| uuid | 1.27 | Apache-2.0 OR MIT | P1 | typed project ids |
| proptest | 1.12 | MIT OR Apache-2.0 | P1 (dev) | undo/redo property test |
| tempfile | 3.27 | MIT OR Apache-2.0 | P1 (dev) | temporary files in tests |
| tracing | 0.1 | MIT | P1 | logging (never in the audio callback) |
| tracing-subscriber | 0.3 | MIT | P1 | log filter and formatting |
| wayland-client | 0.31.15 | MIT | P0 | compositor capability probe |
| ebur128 | 0.1.10 | MIT | planned | LUFS |
| nnnoiseless | 0.5.2 | BSD-3-Clause | planned | RNNoise denoise |
| flacenc | 0.5.1 | Apache-2.0 | P3 | FLAC encoding (`default-features = false`) |
| vorbis_rs | 0.5.6 | BSD-3-Clause | planned | OGG Vorbis encoding |
| rustfft / realfft | 6.4.1 / 3.5.0 | MIT / MIT OR Apache-2.0 | P1 (transitive) | FFT (via rubato) |
| fundsp | 0.23.0 | MIT OR Apache-2.0 | candidate | DSP blocks (own biquads preferred) |

Rejected: `mp3lame-encoder`/LAME (LGPL-3.0), espeak-ng (GPL-3.0),
`espeak-rs` bindings (would link GPL code), Qwen3-TTS-Rust example (no license),
F5-TTS weights (CC-BY-NC), XTTS-v2 (CPML), KittenTTS 2 (custom license + AUP).

## Models

| Model | Role | License (verified) | Status |
|---|---|---|---|
| onnx-community/Kokoro-82M-v1.0-ONNX | main TTS | Apache-2.0 (HF cardData) | P0, ran end-to-end |
| hexgrad/Kokoro-82M (weights origin) | main TTS | Apache-2.0 (HF cardData) | reference |
| KittenML/kitten-tts-nano-0.8-fp32 | instant TTS | Apache-2.0 (model card front matter) | approved, not run |
| onnx-community/moonshine-tiny-ONNX | dictation | MIT (HF cardData) | approved, not run |
| Qwen/Qwen3-TTS-12Hz-0.6B-Base | clone "Fast" | Apache-2.0 (HF cardData) | Phase 8, not run |
| Qwen/Qwen3-TTS-12Hz-1.7B-Base | clone "Best" | Apache-2.0 (HF cardData) | Phase 8, not run |
| openbmb/VoxCPM2 | voice design | Apache-2.0 (HF cardData) | Phase 8, verify weights on model card |
| OmniVoice | clone | **unverified** | blocked until license verified |
| Silero VAD | VAD | MIT | candidate for Phase 7 |

## Data and assets

| Asset | License | Status |
|---|---|---|
| CMU Pronouncing Dictionary v0.7b (`cmudict_data.json`) | BSD-style (CMU) | P0, used via `piper-plus-g2p`; ship the notice with the app |
| Inter variable (`InterVariable.ttf`, family “Inter Variable”) | SIL OFL-1.1 | P2, bundled in `crates/alp-ui-kit/assets/fonts/` with `LICENSE-Inter.txt` |
| Application Brand & Scalable Icon (`io.github.zvirox.AudioLabsPro.svg`) | CC0-1.0 / MIT | P9, bundled in `packaging/icons/` |
| AppStream metadata & MIME spec (`io.github.zvirox.AudioLabsPro.*`) | CC0-1.0 | P9, desktop integration |
| Icons | — | no icon font in Phase 2; Unicode text glyphs render through Inter or the system fallback |

## Distribution obligations

- MPL-2.0 (`symphonia`): ship the license text; changes to its files must be
  published. We do not patch it.
- MIT/BSD/Apache: ship `LICENSE` + this file + the upstream notices.
- Models: ship each `license_url` in the model manifest and show it before
  download; no model is bundled without an accepted license screen.
