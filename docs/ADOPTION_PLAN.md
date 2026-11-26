# Audio Labs Pro — Master Engineering Roadmap & Feature Plan

> **Current Release Baseline:** `v0.1.0-beta.1`
> **Architecture Status:** 12 Rust workspace crates + Tauri v2 + React 19 TypeScript frontend (`frontend/`).
> **UI Policy:** **The Linear UI design system is FINAL and LOCKED.** No glassmorphism, no KWin/compositor blur, no frosted translucent layers, and no alternative UI themes are planned. All frontend changes must pass `npm --prefix frontend run check:linear`.

---

## 1. Completed Milestones (M0–M8 & Phases 0–13)

### Core Architecture & Audio Pipeline
- [x] **12-Crate Clean Boundary Graph:** Verified acyclic architecture (`alp-core` -> `alp-media`/`alp-dsp`/`alp-plugin` -> `alp-engine`/`alp-voice` -> `alp-api`/`alp-session` -> `alp-mcp`/`alp-cli`/`alp-desktop`).
- [x] **Non-Destructive Project & Block Store (`alp-core`):** `.alpproj` project schema, SQLite WAL 64 KB audio sample blocks, clips, fades, gain/pan envelopes, markers, and undo/redo commands.
- [x] **Media Decoding, Peaks & Export (`alp-media`):** Multi-format decoding, multi-resolution peak pyramid cache, high-quality resampling, TPDF dither, and loudness-normalized WAV/FLAC/OGG/MP3 export.
- [x] **Real-Time Audio Engine (`alp-engine`):** Device enumeration, low-latency cpal/PipeWire/ALSA playback & capture, mixer summing, and offline renderer.
- [x] **Voice Cleanup & Spectral Core (`alp-dsp`):** Parametric EQ, biquads, dynamics compressor/gate/de-esser, true-peak limiter, ITU-R BS.1770 LUFS metering, `RnnoiseDenoiser` + `SpectralDenoiser` (`denoise.rs`), and STFT spectrogram + 2D spectral selection (`spectral.rs`).
- [x] **Local Neural Inference & Voice Lab (`alp-infer`, `alp-voice`):** Offline subprocess daemon executing Kokoro/Kitten/StyleTTS2, Qwen3 voice cloning with mandatory written consent attestation, Moonshine STT, VAD, and SHA-256 verified model downloads.
- [x] **CLAP & LV2 Plugin Support (`alp-plugin`):** Plugin discovery and hosting on Linux.
- [x] **MCP Host & Starship Bridge (`alp-mcp`):** 68 authenticated MCP tools (`Observe` / `Edit` / `Export`), UNIX socket host, live UI approval prompts, and Starship Control socket bridge (`$XDG_RUNTIME_DIR/starship/bridges/alp.sock`).

### Desktop Cutover, Linear UI & Release Packaging (Completed M0–M8)
- [x] **M0–M2:** Latency/performance benchmarks verified, `alp-session` extracted as framework-agnostic UI session core, Tauri v2 shell (`alp-desktop`) and Vite + React 19 scaffold (`frontend/`) created.
- [x] **M3–M4:** UI kit (`frontend/src/kit/`), dual-axis navigation shell, real-time transport state synchronization, segmented meters, and zoomable 60 FPS peak-pyramid `WaveformCanvas`.
- [x] **M5–M6:** All workspaces ported (`Dashboard`, `Voices`, `Generate`, `Multitrack`, `Stories`, `Captures`, `Effects`, `Models`, `Settings`) plus non-linear `MultitrackCanvas` and `SpectrogramCanvas`.
- [x] **M7–M8:** Full AI/MCP parity wired into `alp-desktop`, Starship plugin bridge added, legacy `iced` crates (`alp-ui`, `alp-ui-kit`) deleted, legacy manual `deb/`/`rpm/`/`flatpak/` removed in favor of unified `packaging/nfpm.yaml` + `.github/workflows/release.yml` (`.deb`, `.rpm`, `.pkg.tar.zst`).
- [x] **Linear UI Lock:** Finalized shared Linear dark (`#1C1C1C` / `#292929`) and light (`#F8F9FA` / `#FFFFFF`) system with **Magenta** primary accent (`#C026D3` / `#A21CAF`), zero green/teal, chart-free UI (`echarts` removed), self-hosted Inter font, `TitleBar` (`1280x800`, min `1024x640`, sole `ThemeToggle`), collapsible `Sidebar` (`240px`/`56px`), `Breadcrumb`, `LinearCard`/`StatCard`/`DataTable`/`LinearDialog`/`ProgressRow`/`RangeTabs`/`DonutLegend`, and enforced `check:linear` CI guard.

---

## 2. Active Backlog (What Is Truly Pending)

### Phase 14 — Advanced Spectral Repair & Track Insert Racks
- [ ] **Real-Time Track Insert Racks (`alp-engine` & `frontend/`):**
  - [ ] Implement `InsertRack` in `crates/alp-engine/src/insert_rack.rs` (up to 8 live non-destructive insert slots per track with lock-free parameter updates).
  - [ ] Connect `EffectsWorkspace` EQ/dynamics controls directly to active `BiquadChain` magnitude/phase curve telemetry.
- [ ] **Targeted 2D Spectral Repair (`alp-dsp`):**
  - [ ] Implement `SpectralNotchFilter` to attenuate narrow-band tones inside selected `[t1..t2, f1..f2]` spectral boxes.
  - [ ] Implement `HarmonicSuppressor` for automated fundamental + harmonic overtone attenuation.

### Phase 15 — Audiobook Mastering & Native M4B Packaging
- [ ] **ACX Automated QA Report:**
  - [ ] Batch chapter validation checking RMS (`-23` to `-18 dB`), peak ceiling (`<= -3.0 dB`), and floor (`<= -60 dB`) with JSON/UI compliance report.
- [ ] **Native M4B Muxer (`alp-media`):**
  - [ ] Encode AAC audiobook container with `chpl` chapter atoms, cover artwork (JPEG/PNG), and narrator/series metadata tags.

### Phase 16 — Neural Stem Separation & Video Dubbing Pipeline
- [ ] **Stem Separation (`alp-infer`):**
  - [ ] Add ONNX Demucs runner producing 4 isolated stems (`vocals`, `drums`, `bass`, `other`) loaded directly onto multitrack lanes.
- [ ] **WSOLA Rate Fitting & Video Remux (`alp-voice`, `alp-media`):**
  - [ ] Implement `FitPlanner` (`rate_fitting.rs`) time-stretching translated TTS takes to match reference subtitle/dialog windows.
  - [ ] Implement `VideoRemuxer` (`video_sync.rs`) for MP4/MKV audio extraction and lossless video stream copy (`-c:v copy`) remuxing, plus dedicated Dubbing Studio UI controls.

### Phase 17 — VST3 Plugin Host & Batch Macros
- [ ] **VST3 Host Engine (`alp-plugin`):**
  - [ ] Discover and instantiate `.vst3` modules in `/usr/lib/vst3` and `~/.vst3` alongside existing CLAP/LV2 hosts.
  - [ ] Native X11/XWayland external plugin GUI window parenting bridge in `alp-desktop`.
- [ ] **DSP Macro Batch Queue (`alp-engine`, `alp-mcp`):**
  - [ ] Implement `MacroChain` directory batch processing pipeline and expose `dsp_macro_execute` via MCP.

### Phase 18 — Long-Horizon Verification & Fuzzing
- [ ] **Automated 25-Scenario MCP Eval (`mcp_eval.py`):** End-to-end tool sequence benchmarks verifying deterministic state convergence and single-entry undo rollback.
- [ ] **Stability Soak & Fuzzing:** 24-hour continuous playback memory/drop-frame soak test and fuzz harnesses for `.alpproj` JSON/SQLite deserializers and media decoders.
