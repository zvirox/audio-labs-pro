# Technology Stack & Dependencies

Audio Labs Pro combines a pure Rust core audio/ML backend with a Tauri v2 + React 19 TypeScript desktop frontend.

---

## Backend (Rust 2021/2024 Workspace — 12 Crates)

| Layer | Technologies |
|---|---|
| Core Domain & Storage | `serde`, `serde_json`, `rusqlite` (WAL block store), `uuid`, `thiserror`, `tracing` |
| Audio I/O & Engine | `cpal` (PipeWire/ALSA/JACK), `ringbuf`, `crossbeam-channel` |
| Signal Processing & Peaks | `realfft`, `rustfft`, `nnnoiseless` (RNNoise), custom STFT spectral / LUFS / dynamics (`alp-dsp`) |
| Media Codecs & Resampling | `symphonia`, `hound`, `claxon`, `lewton`, `rubato`, custom TPDF dither (`alp-media`) |
| Audio Plugin Hosts | `clack-host` (CLAP), `lilv` / LV2 bindings (`alp-plugin`) |
| Neural Inference Subprocess | `ort` (`onnxruntime`), `llama-cpp-2` (GGUF), Candle (`qwen3.rs`, `kitten.rs`, `kokoro.rs`, `style_tts2.rs`, `moonshine.rs`) |
| AI Control & Starship Suite | `tokio`, JSON-RPC 2.0 MCP server (68 tools), `starship-plugin` Unix socket bridge |
| Desktop Host Shell | `tauri` v2, `tauri-build`, `tokio` IPC bridge |

---

## Frontend (`frontend/` — Locked Linear UI)

> **UI Design Policy:** **Locked & Final.** Built on a clean Linear design system with opaque layered surfaces, solid tokens, zero compositor/CSS blur, and self-hosted Inter typography.

| Concern | Technology |
|---|---|
| Framework & Build | React 19, TypeScript, Vite 8, `@tailwindcss/vite` (Tailwind CSS v4) |
| Window Chrome & Shell | Tauri v2 Window APIs (`TitleBar` 40 px with single `ThemeToggle`, collapsible `Sidebar`, `Breadcrumb`, `SubtabTopBar`, `DockedPlayer`) |
| Icons & Command Palette | `lucide-react`, `cmdk` |
| Charts & Telemetry | `echarts` configured with `echarts-linear-theme.ts` (`getLinearChartPalette`, `buildLinearEChartsOption`) |
| Custom 2D Canvases | Hardware-accelerated HTML5 Canvas 2D (`MultitrackCanvas`, `WaveformCanvas`, `SpectrogramCanvas`) reading CSS variables dynamically on theme change |
| Motion | `motion` + CSS 150 ms `cubic-bezier(0.16, 1, 0.3, 1)` curves, respecting `reduce-motion` |
| Fonts | Self-hosted `InterVariable.ttf` bundled in `frontend/src/assets/fonts/` (`tabular-nums` for telemetry metrics; monospace reserved strictly for timecodes and logs) |
| Enforcement | `frontend/scripts/check-linear.mjs`, Node native test runner + `esbuild` (`timelineMath.test.ts`), Vite production bundle |

---

## Linux Packaging & Distribution

- **Unified Package Builder:** `nfpm` via `packaging/nfpm.yaml` producing `.deb`, `.rpm`, and Arch Linux `.pkg.tar.zst` artifacts.
- **CI & Release:** GitHub Actions on `ubuntu-24.04` (`.github/workflows/ci.yml`, `.github/workflows/release.yml`).
