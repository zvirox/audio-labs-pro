# Tech stack — Phase 1 revision

Replaces Section 4 of the build plan where a choice changed or was confirmed.
Licenses are tracked in `THIRD_PARTY.md`; decisions in `docs/adr/`.

Status: **P1** is in the shipping workspace; **planned** is approved for a later
phase; **deferred** is not in v1.

| Concern | Choice | Version | License | Status | Notes |
|---|---|---|---|---|---|
| Language | Rust | 1.89+ (edition 2024) | — | P1 | one language for app, tests, CLI |
| UI | iced | `=0.14.0` (pinned) | MIT | planned (Phase 2) | wgpu; tokio feature for timers |
| UI kit | in-house `alp-ui-kit` | — | Apache-2.0 | P1 tokens | components in Phase 2 |
| Font | Inter (Regular/Medium/SemiBold) | static | OFL-1.1 | planned | bundle in `assets/fonts/` |
| Motion | `lilt` + in-house spring | 0.8.2 | MIT | planned | tokens 120/200/320 ms |
| Compose blur | vendored `iced_winit` patch + `wayland-client` | 0.31 | MIT | planned (Phase 2) | ADR 0003; fallback always available |
| Audio I/O | `cpal` | 0.18.2 | Apache-2.0 | P1 | ALSA host, F32/I16 |
| Ring buffer | `rtrb` | 0.4.0 | MIT/Apache-2.0 | P1 | SPSC, no locks in callback |
| Decode | `symphonia` | 0.6.1 | MPL-2.0 | P1 | unmodified; WAV/FLAC/MP3/OGG/Vorbis |
| Encode | `hound` | 3.5.1 | Apache-2.0 | P1 | WAV 16/24/f32 |
| Encode | `flacenc`, `vorbis_rs` | 0.5 | Apache / BSD-3 | planned | MP3 deferred (LGPL) |
| Resample | `rubato` | 5.0.1 | MIT/Apache-2.0 | P1 | offline FFT |
| Loudness | `ebur128` | 0.1.10 | MIT | planned | LUFS + true peak |
| Denoise | `nnnoiseless` + classical fallback | 0.5.2 | BSD-3 | planned | speech-oriented |
| FFT | `rustfft` / `realfft` | 6.4 / 3.5 | MIT | P1 (transitive) | spectrogram, spectral tools |
| Inference (light) | `ort` + ONNX Runtime | 2.0.0-rc.15 / 1.31 | MIT/Apache | planned (Phase 6) | Kokoro, Kitten, Moonshine |
| G2P | `piper-plus-g2p` | 0.5.2 | MIT | planned | eSpeak-free, CMUdict |
| Inference (heavy) | `alp-infer` process; llama.cpp/gguf or candle | — | MIT | Phase 8 | ADR 0008 |
| Serialization | `serde` + `serde_json` | 1 | MIT/Apache | P1 | versioned project schema |
| CLI | `clap` + `anyhow` | 4.6 / 1.0 | MIT/Apache | P1 | headless acceptance harness |
| Directories | `dirs` | 7.0 | MIT/Apache | P1 | XDG roots |
| IDs | `uuid` | 1.27 | Apache/MIT | P1 | typed project ids |
| Jobs | `tokio` (timers/IO) + worker threads | 1 | MIT | planned | audio thread stays free of both |
| Logging | `tracing` + `tracing-subscriber` | 0.1 / 0.3 | MIT | P1 | never inside the audio callback |
| Tests | `proptest`, `tempfile` | 1.12 / 3.27 | MIT/Apache | P1 | undo/redo properties, atomic files |

## Rejected or deferred

- Tauri 2 / Slint: rejected for v1 (ADR 0002).
- espeak-ng: GPL, rejected; MIT G2P proven (ADR 0010).
- LAME/MP3 encoder: LGPL, deferred (ADR 0005).
- Python sidecar for models: rejected (packaging and rule 0.3).
- `iced_plus`, `iced_ui`: not adopted; `iced_aw` optional (ADR 0007).
