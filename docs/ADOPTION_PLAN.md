# Audio Labs Pro — Master Engineering Roadmap & Feature Plan

> **Current Release Baseline:** `v0.1.0-beta.4`
> **Architecture Status:** 12 Rust workspace crates + Tauri v2 + React 19 TypeScript frontend (`frontend/`).
> **UI Policy:** **The Linear UI design system is FINAL and LOCKED.** No glassmorphism, no KWin/compositor blur, no frosted translucent layers, and no alternative UI themes are planned. All frontend changes must pass `npm --prefix frontend run check:linear`.

---

## 1. Completed Milestones (M0–M8 & Phases 0–13)

### Core Architecture & Audio Pipeline
- [x] **12-Crate Clean Boundary Graph:** Verified acyclic architecture (`alp-core` -> `alp-media`/`alp-dsp`/`alp-plugin` -> `alp-engine`/`alp-voice` -> `alp-api`/`alp-session` -> `alp-mcp`/`alp-cli`/`alp-desktop`).
- [x] **Non-Destructive Project & Block Store (`alp-core`):** `*.alp/` project directory schema (`project.json`), checksummed `65,536`-frame audio sample blocks (no SQLite engine), clips, fades, gain/pan envelopes, markers, and undo/redo commands.
- [x] **Media Decoding, Peaks & Export (`alp-media`):** Multi-format decoding, multi-resolution peak pyramid cache, high-quality resampling, TPDF dither, and loudness-normalized WAV/FLAC/OGG/MP3 export.
- [x] **Real-Time Audio Engine (`alp-engine`):** Device enumeration, low-latency cpal/PipeWire/ALSA playback & capture, mixer summing, and offline renderer.
- [x] **Voice Cleanup & Spectral Core (`alp-dsp`):** Parametric EQ, biquads, dynamics compressor/gate/de-esser, true-peak limiter, ITU-R BS.1770 LUFS metering, `RnnoiseDenoiser` + `SpectralDenoiser` (`denoise.rs`), and STFT spectrogram + 2D spectral selection (`spectral.rs`).
- [x] **Local Neural Inference & Voice Lab (`alp-infer`, `alp-voice`):** Offline subprocess daemon executing Kokoro/Kitten/StyleTTS2, Qwen3 voice cloning with mandatory written consent attestation, Moonshine STT, VAD, and SHA-256 verified model downloads.
- [x] **CLAP, LV2 & VST3 Plugin Support (`alp-plugin`):** Plugin discovery, C/COM hosting, automation, and X11 editor embedding on Linux.
- [x] **MCP Host & Starship Bridge (`alp-mcp`):** 87 authenticated MCP tools (`80` in `tool_defs.rs` + `7` live `transport_*` in `transport_tools.rs`, across `Observe` / `Edit` / `Export`), UNIX socket host, live UI approval prompts, and Starship Control socket bridge (`$XDG_RUNTIME_DIR/starship/bridges/alp.sock`).

### Desktop Cutover, Linear UI & Release Packaging (Completed M0–M8)
- [x] **M0–M2:** Latency/performance benchmarks verified, `alp-session` extracted as framework-agnostic UI session core, Tauri v2 shell (`alp-desktop`) and Vite + React 19 scaffold (`frontend/`) created.
- [x] **M3–M4:** UI kit (`frontend/src/kit/`), dual-axis navigation shell, real-time transport state synchronization, segmented meters, and zoomable 60 FPS peak-pyramid `WaveformCanvas`.
- [x] **M5–M6:** All workspaces ported (`Dashboard`, `Voices`, `Generate`, `Multitrack`, `Stories`, `Captures`, `Effects`, `Models`, `Settings`) plus non-linear `MultitrackCanvas` and `SpectrogramCanvas`.
- [x] **M7–M8:** Full AI/MCP parity wired into `alp-desktop`, Starship plugin bridge added, legacy `iced` crates (`alp-ui`, `alp-ui-kit`) deleted, legacy manual `deb/`/`rpm/`/`flatpak/` removed in favor of unified `packaging/nfpm.yaml` + `.github/workflows/release.yml` (`.deb`, `.rpm`, `.pkg.tar.zst`).
- [x] **Linear UI Lock:** Finalized shared Linear dark (`#1C1C1C` / `#292929`) and light (`#F8F9FA` / `#FFFFFF`) system with **Magenta** primary accent (`#C026D3` / `#A21CAF`), zero green/teal, chart-free UI (`echarts` removed), self-hosted Inter font, `TitleBar` (`1280x800`, min `1024x640`, sole `ThemeToggle`), collapsible `Sidebar` (`240px`/`56px`), `Breadcrumb`, `LinearCard`/`StatCard`/`DataTable`/`LinearDialog`/`ProgressRow`/`RangeTabs`/`DonutLegend`, and enforced `check:linear` CI guard.

---

## 2. Completed Adoption Phases & Post-Roadmap (Released in v0.1.0-beta.4)

### Phase 14 — Advanced Spectral Repair & Track Insert Racks (released in v0.1.0-beta.4)
- [x] **Real-Time Track Insert Racks (`alp-engine` & `frontend/`):**
  - [x] Implement `InsertRack` in `crates/alp-engine/src/insert_rack.rs` (up to 8 live non-destructive insert slots per track with lock-free parameter updates).
  - [x] Connect `EffectsWorkspace` EQ/dynamics controls directly to active `BiquadChain` magnitude/phase curve telemetry.
- [x] **Targeted 2D Spectral Repair (`alp-dsp`):**
  - [x] Implement `SpectralNotchFilter` to attenuate narrow-band tones inside selected `[t1..t2, f1..f2]` spectral boxes.
  - [x] Implement `HarmonicSuppressor` for automated fundamental + harmonic overtone attenuation.

### Phase 15 — Audiobook Mastering & Native M4B Packaging (released in v0.1.0-beta.4)
- [x] **ACX Automated QA Report:**
  - [x] Batch chapter validation checking RMS (`-23` to `-18 dB`), peak ceiling (`<= -3.0 dB`), and floor (`<= -60 dB`) with JSON/UI compliance report.
- [x] **Native M4B Muxer (`alp-media`):**
  - [x] Encode AAC audiobook container with `chpl` chapter atoms, cover artwork (JPEG/PNG), and narrator/series metadata tags.

### Phase 16 — Neural Stem Separation & Video Dubbing Pipeline (released in v0.1.0-beta.4)
- [x] **Stem Separation (`alp-infer`):**
  - [x] Add ONNX Demucs runner producing 4 isolated stems (`vocals`, `drums`, `bass`, `other`) loaded directly onto multitrack lanes.
- [x] **WSOLA Rate Fitting & Video Remux (`alp-voice`, `alp-media`):**
  - [x] Implement `FitPlanner` (`rate_fitting.rs`) time-stretching translated TTS takes to match reference subtitle/dialog windows.
  - [x] Implement `VideoRemuxer` (`video_sync.rs`) for MP4/MKV audio extraction and lossless video stream copy (`-c:v copy`) remuxing, plus dedicated Dubbing Studio UI controls.

### Phase 17 — VST3 Plugin Host & Batch Macros (completed on main post-v0.1.0-beta.4)
- [x] **VST3 Host Engine (`alp-plugin`):**
  - [x] Discover `.vst3` bundles and instantiate real Linux `.vst3` plugins via Steinberg C++ COM ABI (`vst3 = 0.3.0`: `IComponent`, `IAudioProcessor`, `IEditController`, parameter automation queues, `IPlugView`), verified with compiled C shared-library tests (`tests/vst3_host.rs`).
  - [x] Native X11/XWayland external plugin GUI window parenting bridge (`X11EmbedWindowID`) in `alp-desktop` & `alp-plugin`.
- [x] **DSP Macro Batch Queue (`alp-engine`, `alp-mcp`):**
  - [x] Implement `MacroChain` directory batch processing pipeline and expose `dsp_macro_execute` via MCP.

### Phase 18 — Long-Horizon Verification & Fuzzing (released in v0.1.0-beta.4)
- [x] **Automated 25-Scenario MCP Eval (`mcp_eval.py`):** End-to-end tool sequence benchmarks verifying deterministic state convergence and single-entry undo rollback.
- [x] **Stability Soak & Fuzzing:** Callback-endurance playback soak (`#[ignore]` 24h virtual-clock run in `long_horizon_soak.rs`) and proptest fuzz harnesses for `project.json` / `SampleBlock` serde+checksums and media decoders.

### Phase 19 — Release Readiness & MCP Coverage (released in v0.1.0-beta.4)
- [x] **Studio MCP Coverage:** `effects_response`, `acx_qa_analyze`, `audiobook_export_m4b`, `marker_remove`, `loop_range_set` and `clip_trim` brought the `tool_defs.rs` surface to 75 tools at Phase 19 (later extended to 80 + 7 live transport tools = 87 total), with end-to-end Suite 9 checks.
- [x] **Packaging Parity:** `alp-mcp` is built by `release.yml` and installed by `nfpm.yaml`; `scripts/phase19-release-check.sh` performs a local deb/rpm/archlinux dry-run and SHA256SUMS verification.

### Post-Roadmap — Extended MCP Coverage, Live Transport, Demucs Catalog & Ubuntu 22.04 Compat
- [x] Extended Export tools in `tool_defs.rs` (`dubbing_fit_rate`, `dubbing_remux`, `dsp_spectral_repair`, `dsp_time_stretch`, plus `dubbing_separate_stems` backed by `alp_voice::InferClient` and MIT `demucs-4stem` download manifest) reached 80 tools with Suite 10 checks.
- [x] Seven live transport tools in `transport_tools.rs` (`transport_state`, play/pause/stop/seek, volume, loop) brought the total MCP surface to **87 tools**, handled on the desktop UI thread through the existing MCP control envelope (headless mode fails honestly).
- [x] Pinned `ort = "=2.0.0-rc.10"` (ORT 1.22, no `GLIBC_2.38+` symbols) and restored `.github/workflows/release.yml` to `ubuntu-22.04` with Ubuntu 22.04 container smoke testing enabled.

---

## 3. Truly Pending Work & Honest Limits

- [ ] **Version Bump & 1.0 Release Cut:** Promotion from pre-release `v0.1.0-beta.4` to `v1.0.0-rc.1` / `v1.0.0` (workspace/Tauri/frontend versions and non-prerelease release cut, intentionally deferred by user).
- **Operational Runtime Limits:** `dubbing_separate_stems` and `demucs-4stem` download weights on demand via `ModelManager` (not bundled inside the installer) and require a running `alp-infer` daemon; X11 `IPlugView` embedding targets X11/XWayland rather than native Wayland surfaces.
