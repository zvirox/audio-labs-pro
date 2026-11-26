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
| tauri | 2.12.7 | MIT OR Apache-2.0 | M2 | desktop application framework, IPC runtime, and WebKitGTK wrapper (`alp-desktop`) |
| tauri-build | 2.7.6 | MIT OR Apache-2.0 | M2 | compile-time asset packager and build code generator |
| raw-window-handle | 0.6.2 | MIT OR Apache-2.0 | M2 | native window handle interoperability |
| nix | 0.30.1 | MIT | P1 | safe Linux peer credentials and effective-UID checks for the UI control socket |
| x11rb | 0.13.2 | MIT OR Apache-2.0 | P2 | Safe X11 display inspection utilities |
| cpal | 0.18.2 | Apache-2.0 | P1 | audio I/O |
| rtrb | 0.4.0 | MIT OR Apache-2.0 | P1 | lock-free SPSC ring buffer |
| symphonia | 0.6.1 | MPL-2.0 | P1 | decoding (unmodified; file-level copyleft only) |
| hound | 3.5.1 | Apache-2.0 | P1 | WAV read/write |
| mp4 | 0.14.0 | MIT | P1 | ISO-BMFF/M4B container writer |
| oxideav-aac | 0.1.7 | MIT | P1 | Pure-Rust AAC-LC encoder for M4B audio |
| bytes | 1.x | MIT | P1 | Byte buffers for MP4 samples |
| rubato | 5.0.1 | MIT OR Apache-2.0 | P1 | offline FFT resampling |
| crane-core | 26.9.0 | MIT | P0 | validated Candle Qwen3-TTS neural cloning runtime, pinned exactly |
| ort | 2.0.0-rc.15 | MIT OR Apache-2.0 | P0 | ONNX Runtime bindings; Phase 6 |
| ONNX Runtime (binary) | 1.31 | MIT | P0 | downloaded by `ort` |
| piper-plus-g2p | 0.5.2 | MIT | P0 | English G2P (eSpeak-free) |
| ureq | 3.5.0 | MIT OR Apache-2.0 | P0 | blocking HTTPS model downloads (rustls, resumable) |
| fs4 | 1.1.0 | MIT OR Apache-2.0 | P1 | free-disk-space checks (pure-Rust statvfs) |
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
| nnnoiseless | 0.5.2 | BSD-3-Clause | P5 | RNNoise denoise |
| flacenc | 0.5.1 | Apache-2.0 | P3 | FLAC encoding (`default-features = false`) |
| vorbis_rs | 0.5.6 | BSD-3-Clause | planned | OGG Vorbis encoding |
| rustfft / realfft | 6.4.1 / 3.5.0 | MIT / MIT OR Apache-2.0 | P5 | FFT (rubato, alp-dsp spectral denoiser) |
| fundsp | 0.23.0 | MIT OR Apache-2.0 | candidate | DSP blocks (own biquads preferred) |

## Retired Rust crates (superseded in M8 cutover)

| Crate | Former Version | License | Status | Replacement |
|---|---|---|---|---|
| iced | 0.14.0 | MIT | Retired in M8 | Replaced by Tauri 2 + React webview desktop shell (`alp-desktop`) |
| iced-cube | 0.0.6 | MIT OR Apache-2.0 | Retired in M8 | Replaced by Tailwind CSS and React kit components (`frontend/src/kit`) |
| lucide-static-svg | 1.62.0 | ISC | Retired in M8 | Replaced by `lucide-react` in frontend |
| lilt | 0.8.2 | MIT | Retired in M8 | Replaced by CSS transitions and React motion handling |

## Linux system runtimes

| Library / Package | Version | License | Status | Purpose |
|---|---|---|---|---|
| WebKitGTK 4.1 (`libwebkit2gtk-4.1-0`) | >= 2.40.0 | LGPL-2.1+ / BSD | M0 / M8 | Native WebKit browser engine powering Tauri 2 desktop GUI |
| GTK 3 (`libgtk-3-0`) | >= 3.24.0 | LGPL-2.0+ | M0 / M8 | Windowing and display backend for WebKitGTK |

## Frontend npm packages

| Package | Version | License | Status | Purpose |
|---|---|---|---|---|
| @tauri-apps/api | ^2.12.5 | MIT OR Apache-2.0 | M2 | Tauri frontend IPC client bindings and channel listener |
| react | ^19.3.1 | MIT | M2 | Studio component tree and reactive state model |
| react-dom | ^19.3.1 | MIT | M2 | Web DOM rendering |
| cmdk | ^1.1.1 | MIT | M3 | Accessible, unstyled command palette primitive (`Ctrl+K`) |
| lucide-react | ^1.78.0 | ISC | M3 | Clean, consistent studio iconography |
| tailwindcss | ^4.3.5 | MIT | M2 | Utility-first CSS engine powering locked opaque Linear UI tokens |
| @tailwindcss/vite | ^4.3.5 | MIT | M2 | Vite integration plugin for Tailwind CSS v4 |
| typescript | ^7.0.2 | Apache-2.0 | M2 | Static type safety and IPC interface contracts |
| vite | ^8.3.8 | MIT | M2 | Fast ESM frontend bundler and development server |
| @vitejs/plugin-react | ^6.3.0 | MIT | M2 | Babel/Fast Refresh React plugin for Vite |
| esbuild | ^0.28.2 | MIT | M2 | Fast TypeScript test runner bundler |

Rejected: `mp3lame-encoder`/LAME (LGPL-3.0), espeak-ng (GPL-3.0),
`espeak-rs` bindings (would link GPL code), Qwen3-TTS-Rust example (no license),
F5-TTS weights (CC-BY-NC), XTTS-v2 (CPML), KittenTTS 2 (custom license + AUP).

## Models

| Model | Role | License (verified) | Status |
|---|---|---|---|
| onnx-community/Kokoro-82M-v1.0-ONNX | main TTS | Apache-2.0 (HF cardData) | P0, ran end-to-end |
| hexgrad/Kokoro-82M (weights origin) | main TTS | Apache-2.0 (HF cardData) | reference |
| KittenML/kitten-tts-nano-0.8-fp32 | instant TTS | Apache-2.0 (model card front matter) | approved, not run |
| onnx-community/kitten-tts-nano-0.1-ONNX | instant TTS (downloadable via `kitten-nano` manifest) | Apache-2.0 (HF cardData) | P6, files hash-verified, engine not wired |
| onnx-community/moonshine-tiny-ONNX | dictation | MIT (HF cardData) | approved, not run |
| Qwen/Qwen3-TTS-12Hz-0.6B-Base | clone "Fast" | Apache-2.0 (HF cardData) | P0, real CPU neural inference through daemon |
| Qwen/Qwen3-TTS-12Hz-1.7B-Base | clone "Best" | Apache-2.0 (HF cardData) | Phase 8, not run |
| openbmb/VoxCPM2 | voice design | Apache-2.0 (HF cardData) | Phase 8, verify weights on model card |
| OmniVoice | clone | **unverified** | blocked until license verified |
| Silero VAD | VAD | MIT | candidate for Phase 7 |

## Data and assets

| Asset | License | Status |
|---|---|---|
| CMU Pronouncing Dictionary v0.7b (`cmudict_data.json`) | BSD-style (CMU) | P0, used via `piper-plus-g2p`; ship the notice with the app |
| Inter variable (`InterVariable.ttf`, family “Inter Variable”) | SIL OFL-1.1 | Bundled in `frontend/src/assets/fonts/` with `LICENSE-Inter.txt` |
| Application Brand & Scalable Icon (`audio-labs-pro.png` / `.desktop`) | CC0-1.0 / MIT | Bundled in `packaging/icons/` & `packaging/` |
| AppStream & MIME spec (`packaging/audio-labs-pro.xml`) | CC0-1.0 | Linux desktop integration via `packaging/nfpm.yaml` |
| Lucide icons | ISC | Bundled via `lucide-react` npm package |

## Distribution obligations

- MPL-2.0 (`symphonia`): ship the license text; changes to its files must be
  published. We do not patch it.
- MIT/BSD/Apache: ship `LICENSE` + this file + the upstream notices.
- Models: ship each `license_url` in the model manifest and show it before
  download; no model is bundled without an accepted license screen.
