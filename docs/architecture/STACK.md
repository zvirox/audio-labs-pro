# Tech stack — Phase 3 revision

Replaces Section 4 of the build plan where a choice changed or was confirmed.
Licenses are tracked in `THIRD_PARTY.md`; decisions in `docs/adr/`.

Status: **P3** is in the shipping workspace; **planned** is approved for a later
phase; **deferred** is not in v1.

| Concern | Choice | Version | License | Status | Notes |
|---|---|---|---|---|---|
| Language | Rust | 1.89+ (edition 2024) | — | P2 | one language for app, tests, CLI |
| UI | iced | `=0.14.0` (pinned) | MIT | P2 | wgpu renderer, `canvas` + `advanced`; no tokio |
| UI kit | in-house `alp-ui-kit` | — | Apache-2.0 | P2 | tokens, Inter, controls, overlays, motion, waveform |
| Font | Inter variable | bundled `InterVariable.ttf` | OFL-1.1 | P2 | family name `Inter Variable`; OFL text bundled |
| Motion | `lilt` | 0.8.2 | MIT | P2 | 120/200/320 ms; panel open/close offset |
| X11 blur | `x11rb` | 0.13.2 | MIT/Apache-2.0 | P2 | KWin `_KDE_NET_WM_BLUR_BEHIND_REGION`; safe Rust |
| Wayland blur | `ext_background_effect_v1` | — | — | deferred | foreign-display bridge requires unsafe; ADR 0012 |
| Audio I/O | `cpal` | 0.18.2 | Apache-2.0 | P1 | ALSA host, F32/I16 |
| Ring buffer | `rtrb` | 0.4.0 | MIT/Apache-2.0 | P1 | SPSC, no locks in callback |
| Decode | `symphonia` | 0.6.1 | MPL-2.0 | P1 | unmodified; WAV/FLAC/MP3/OGG/Vorbis |
| Encode | `hound` | 3.5.1 | Apache-2.0 | P1 | WAV 16/24/f32, RIFF tags |
| Encode | `flacenc` | 0.5.1 | Apache-2.0 | P3 | FLAC 16/24-bit, Vorbis comments |
| Encode | `vorbis_rs` | 0.5.6 | BSD-3 | planned | OGG Vorbis; MP3 deferred (LGPL) |
| Resample | `rubato` | 5.0.1 | MIT/Apache-2.0 | P1 | offline FFT |
| Loudness | `ebur128` | 0.1.10 | MIT | planned | LUFS + true peak |
| Denoise | `nnnoiseless` + classical fallback | 0.5.2 | BSD-3 | planned | speech-oriented |
| FFT | `rustfft` / `realfft` | 6.4 / 3.5 | MIT | P1 (transitive) | spectrogram, spectral tools |
| Inference (light) | `ort` + ONNX Runtime | 2.0.0-rc.15 / 1.31 | MIT/Apache | planned (Phase 6) | Kokoro, Kitten, Moonshine |
| G2P | `piper-plus-g2p` | 0.5.2 | MIT | planned | eSpeak-free, CMUdict |
| Inference (heavy) | `alp-infer` process; llama.cpp/gguf or candle | — | MIT | Phase 8 | ADR 0008 |
| Serialization | `serde` + `serde_json` | 1 | MIT/Apache | P1 | versioned project/workspace schemas |
| CLI | `clap` + `anyhow` | 4.6 / 1.0 | MIT/Apache | P1 | headless acceptance harness |
| Directories | `dirs` | 7.0 | MIT/Apache | P1 | XDG roots |
| IDs | `uuid` | 1.27 | Apache/MIT | P1 | typed project ids |
| Background jobs | `std::thread` + `mpsc` + atomics | std | — | P2 | cancellable jobs; real background mix export in P3; no async runtime |
| Capture recovery | raw f32 spool + JSON manifest | — | — | P3 | chunk-boundary sync; `recover_capture` |
| Dither / null test | in-house | — | Apache-2.0 | P3 | deterministic 1-LSB TPDF; sample-difference report |
| Logging | `tracing` + `tracing-subscriber` | 0.1 / 0.3 | MIT | P1 | never inside the audio callback |
| Tests | `proptest`, `tempfile` | 1.12 / 3.27 | MIT/Apache | P1 | undo/redo properties, atomic files |

## Rejected or deferred

- Tauri 2 / Slint: rejected for v1 (ADR 0002).
- espeak-ng: GPL, rejected; MIT G2P proven (ADR 0010).
- LAME/MP3 encoder: LGPL, deferred (ADR 0005).
- Python sidecar for models: rejected (packaging and rule 0.3).
- `iced_plus`, `iced_ui`: not adopted; `iced_aw` optional (ADR 0007).
- Vendored `iced_winit` blur patch: no longer needed for handles. iced 0.14's
  `window::run` exposes them; only the Wayland foreign-display attachment remains
  blocked by the unsafe-code policy (ADR 0012).
- Tokio in the UI: not adopted. Conditional `window::frames()` plus worker
  channels cover Phase 2 without an async runtime.
