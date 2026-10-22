# Tech stack — Phase 0 revision

Replaces Section 4 of the build plan where a choice changed or was confirmed.
Licenses are tracked in `THIRD_PARTY.md`; decisions in `docs/adr/`.

| Concern | Choice | Version | License | Notes |
|---|---|---|---|---|
| Language | Rust | 1.99 (edition 2024) | — | one language for app, tests, CLI |
| UI | iced | `=0.14.0` (pinned) | MIT | wgpu; tokio feature for timers |
| UI kit | in-house `alp-ui-kit` | — | Apache-2.0 | iced_aw 0.14.1 only behind wrappers |
| Font | Inter (Regular/Medium/SemiBold) | static | OFL-1.1 | bundle in `assets/fonts/` |
| Motion | `lilt` + in-house spring | 0.8.2 | MIT | tokens 120/200/320 ms |
| Compose blur | vendored `iced_winit` patch + `wayland-client` | 0.31 | MIT | ADR 0003; fallback always available |
| Audio I/O | `cpal` | 0.18.2 | Apache-2.0 | ALSA host, F32/I16 |
| Ring buffer | `rtrb` | 0.4.0 | MIT/Apache-2.0 | SPSC, no locks in callback |
| Decode | `symphonia` | 0.6.1 | MPL-2.0 | unmodified; WAV/FLAC/MP3/OGG/Vorbis |
| Encode | `hound`, `flacenc`, `vorbis_rs` | 3.5 / 0.5 / 0.5 | Apache / Apache / BSD-3 | MP3 deferred (LGPL) |
| Resample | `rubato` | 5.0.1 | MIT/Apache-2.0 | pinned |
| Loudness | `ebur128` | 0.1.10 | MIT | LUFS + true peak |
| Denoise | `nnnoiseless` + classical fallback | 0.5.2 | BSD-3 | speech-oriented |
| FFT | `rustfft` / `realfft` | 6.4 / 3.5 | MIT | spectrogram, spectral tools |
| Inference (light) | `ort` + ONNX Runtime | 2.0.0-rc.15 / 1.31 | MIT/Apache | Kokoro, Kitten, Moonshine |
| G2P | `piper-plus-g2p` | 0.5.2 | MIT | eSpeak-free, CMUdict |
| Inference (heavy) | `alp-infer` process; llama.cpp/gguf or candle | — | MIT | deferred to Phase 8 (ADR 0008) |
| Serialization | `serde` + `serde_json` | 1 | MIT/Apache | versioned project schema |
| Jobs | `tokio` (timers/IO) + worker threads | 1 | MIT | audio thread stays free of both |
| Logging | `tracing` | 0.1 | MIT | never inside the audio callback |

## Rejected or deferred

- Tauri 2 / Slint: rejected for v1 (ADR 0002).
- espeak-ng: GPL, rejected; MIT G2P proven (ADR 0010).
- LAME/MP3 encoder: LGPL, deferred (ADR 0005).
- Python sidecar for models: rejected (packaging and rule 0.3).
- `iced_plus`, `iced_ui`: not adopted; `iced_aw` optional (ADR 0007).
