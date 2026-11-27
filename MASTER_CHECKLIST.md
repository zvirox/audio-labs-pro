# Audio Labs Pro — Master Verification & Engineering Checklist

> Last verified against codebase: `v0.1.0-beta.4` (12 Rust crates, `frontend/` Tauri v2 + React 19 Linear UI, `packaging/nfpm.yaml`).
> **UI Policy:** The **Linear UI** (shared Tessera-sampled matte palette `#1C1C1C` dark / `#F8F9FA` light, **Magenta** primary accent `#C026D3`/`#A21CAF`, zero green/teal, chart-free, self-hosted Inter, `check:linear` enforced) is **final and locked** per [DESIGN.md](DESIGN.md). No further UI style redesigns are planned.

---

## Executive Summary

| Track | Status | Verified Coverage |
|---|---|---|
| Core Project Model & Block Store (`alp-core`) | **Complete** | Non-destructive clips, fades, gain/pan envelopes, markers, `*.alp/` JSON + checksummed `SampleBlock` persistence, undo/redo commands |
| Media Codecs, Peaks & Export (`alp-media`) | **Complete** | WAV/FLAC/OGG/MP3 decoding, multi-res peak pyramid, resampling, TPDF dither, loudness export, WSOLA time-stretch, native M4B & FFmpeg remux |
| Real-Time Audio Engine (`alp-engine`) | **Complete** | cpal/PipeWire/ALSA I/O, lock-free mixer, 8-slot `InsertRack`, transport, scene state, offline renderer, `MacroChain` batcher |
| DSP & Spectral Engine (`alp-dsp`) | **Core Complete** | Parametric EQ, biquads, compressor/gate/de-esser, true-peak limiter, LUFS meter, RNNoise + spectral denoiser (`denoise.rs`), STFT spectrogram & 2D `repair.rs` |
| Voice & Neural Inference (`alp-voice`, `alp-infer`) | **Core Complete** | Kokoro/Kitten/StyleTTS2, Qwen3-TTS cloning + mandatory consent attestation, Moonshine STT, VAD, pronunciation lexicon, GGUF/ONNX download & SHA-256 verification, model-gated Demucs 4-stem runner |
| Audio Plugins (`alp-plugin`) | **Complete (CLAP, LV2 & VST3)** | System scanner (`~/.clap`, `/usr/lib/clap`, LV2, `~/.vst3`, `/usr/lib/vst3`), CLAP/LV2 + VST3 COM ABI hosting (`IComponent`/`IAudioProcessor`/`IEditController`/`IPlugView`), X11 editor bridge, presets |
| Session Bridge & AI Control (`alp-session`, `alp-mcp`, `alp-cli`) | **Complete** | Tauri IPC commands, 87 MCP tools (`80` in `tool_defs.rs` + `7` `transport_*` in `transport_tools.rs`) across Observe/Edit/Export tiers, Starship Control bridge (`alp.sock`), live UNIX socket host |
| Desktop Shell & Locked Linear UI (`alp-desktop`, `frontend/`) | **Complete & Locked** | Tauri v2 frameless window (`1280x800`, min `1024x640`, opaque solid surface), TitleBar/Sidebar/Breadcrumb shell, 9 workspaces, Multitrack + Spectrogram canvases, chart-free engine telemetry, `check:linear` gate |
| Packaging & Release (`packaging/nfpm.yaml`, `.github/workflows/`) | **Complete** | Multi-distro `.deb`, `.rpm`, and Arch `.pkg.tar.zst` via `nfpm` on `ubuntu-22.04` (`glibc 2.35+` / ORT 1.22); CI + `v0.1.0-beta.4` prerelease pipeline |

---

## Completed Foundation & Desktop Tracks (Phases 0–19, M0–M8)

- [x] **Phase 0–1 (Core, Engine & Media):** 12-crate Cargo workspace (`alp-core`, `alp-media`, `alp-dsp`, `alp-plugin`, `alp-engine`, `alp-voice`, `alp-infer`, `alp-api`, `alp-mcp`, `alp-session`, `alp-desktop`, `alp-cli`), non-destructive `*.alp/` project directories (`project.json`), checksummed `65,536`-frame sample blocks, lock-free ring buffers, offline master/stem renderers, CI dependency/doc/length guards.
- [x] **Phase 2–3 (Capture, Editing & Timeline Foundation):** Live mic capture & VU metering, multi-resolution peak pyramids, selection/split/trim/slip/nudge/fade commands, full undo/redo command stack.
- [x] **Phase 4 (Multitrack Arranger & Mixer):** Multitrack track lanes, gain/pan faders, mute/solo/arm, bus routing, interactive HTML5 Canvas 2D timeline (`MultitrackCanvas.tsx`), playhead scrubbing, magnetic snapping, ripple edit mode, gain/pan clip envelopes, cue markers.
- [x] **Phase 4.7 & Phase 13 (Spectrogram & Multi-View):** `alp-dsp::spectral` STFT FFT engine, per-track `Waveform | Spectrogram | Dual` view modes, calibrated heatmap palettes (Magma, Inferno, Viridis), interactive 2D bounding-box spectral selection with `fc` / `Q` readouts (`SpectrogramCanvas.tsx`).
- [x] **Phase 5 (DSP Chain & Voice Cleanup):** 10-band EQ (`eq.rs`), biquad filters, dynamics compressor/limiter, EBU R128 / ITU-R BS.1770 LUFS metering (`loudness.rs`), voice chain presets (`voice.rs`), `RnnoiseDenoiser`, `SpectralDenoiser`, and `NoiseReduction` (`denoise.rs`).
- [x] **Phase 6–8 (Models, STT & Voice Cloning):** `alp-infer` runners (`kokoro`, `kitten`, `style_tts2`, `qwen3`, `moonshine`, `gguf`), hardware probe, resumable verified downloads, voice cloning consent attestation gate, multi-speaker audiobook script model & TTS stitching.
- [x] **Phase 10 & Post-Roadmap (MCP Host, Starship & Transport Bridge):** Initial MCP server foundation (`Observe` / `Edit` / `Export` permission tiers, token auth, UI attach approval flow, audit JSONL log, `starship.rs` bridge on `$XDG_RUNTIME_DIR/starship/bridges/alp.sock`), expanded across Phases 13–19, Extended Coverage, and Live Transport Bridge to **87 typed tools** (`80` in `tool_defs.rs` + `7` `transport_*` in `transport_tools.rs`).
- [x] **Phase 11 (Plugin Hosting):** CLAP, LV2, and VST3 plugin discovery and real-time host integration in `alp-plugin`.
- [x] **M0–M8 (Tauri v2 Migration & Legacy Removal):** Full cutover from `iced` (`alp-ui` / `alp-ui-kit` removed) to Tauri v2 + React 19 + TypeScript; unified Linux packaging via `packaging/nfpm.yaml` (`.deb`, `.rpm`, `.pkg.tar.zst`).
- [x] **Phase 14 (Spectral Repair & Insert Racks):** `alp-dsp::repair` notch/harmonic STFT effects, `alp-engine::insert_rack` with 8 lock-free controlled slots, and `EffectsWorkspace` native magnitude/phase curve from real `Coefficients::response` telemetry.
- [x] **Phase 15 (Audiobook Mastering):** ACX chapter QA with quarter-sample true-peak estimation, native M4B export with AAC-LC/chapters/cover/tags, and UI/IPC exposure in `StoriesWorkspace`.
- [x] **Phase 16 (Dubbing Pipeline):** WSOLA time-stretch + `FitPlanner`, external ffmpeg stream-copy remux, downloadable MIT `demucs-4stem` manifest + packed HTDemucs ONNX runner (`DemucsEngine`), Export-tier `dubbing_separate_stems` MCP tool (`InferClient`), and `DubbingWorkspace`.
- [x] **Phase 17 (VST3 Host Runtime & Batch Macros):** `.vst3` bundle discovery + C++ COM ABI instantiation/processing/automation (`IComponent`/`IAudioProcessor`/`IEditController`/`IPlugView`), `x11rb` X11 editor reparenting bridge, offline `MacroChain` batches, and `dsp_macro_execute` MCP job tool.
- [x] **Phase 19 & Extended MCP Coverage:** Studio/dubbing/repair MCP wrappers reached `75` -> `79` -> `80` tools in `tool_defs.rs` (plus `7` `transport_*` tools in `transport_tools.rs` = `87` total); `alp-mcp` built and packaged in `nfpm.yaml`; `scripts/phase19-release-check.sh` verifies local packages and `SHA256SUMS`.
- [x] **Linear UI Finalization & Lock:**
  - [x] `TitleBar` (40 px, app mark + version from `tauri.conf.json`, sole `ThemeToggle` storing `ui-theme`, clean drag region, minimize/maximize/`#E5484D` close).
  - [x] Collapsible `Sidebar` (240 px / 56 px, 36 px items, workspace switcher), `Breadcrumb`, `SubtabTopBar`, `InspectorDrawer`, `DockedPlayer`, `CommandPalette` (cmdk), `ShortcutsModal`.
  - [x] Shared matte palette tokens (`styles/linear-theme.css` + Magenta `styles/app-accent.css`, `#1C1C1C`/`#292929` dark, `#F8F9FA`/`#FFFFFF` light, zero green/teal), self-hosted Inter font, `tabular-nums` stats/tables, flat discrete segmented `Meter` strips (`--sky` -> `--amber` -> `--red`).
  - [x] Zero compositor blur (`glass.rs` forces opaque solid surface; KWin X11 blur removed), zero translucent fills, zero glow/gradients, zero dark shadows, verified by `npm run check:linear`.

---

## Remaining Work & Known Limits

### Verification, Soak & QA
- [x] Named MCP evaluation suite with convergence, single-entry undo and full rollback verification (`scripts/mcp_eval_scenarios.py`).
- [x] Callback endurance playback soak (`#[ignore]` 24h virtual-clock test), JSON `SampleBlock`/project and media decoder proptest fuzz targets (`scripts/phase18-demo.sh`).

### Open Gaps / Release Cut
- [x] **VST3 COM ABI Host Runtime:** Implemented in `alp-plugin::vst3` via `vst3 = 0.3.0` (`ModuleEntry`, `GetPluginFactory`, `IComponent`, `IAudioProcessor`, `IEditController`, `"X11EmbedWindowID"` `IPlugView`) and verified against a real C COM shared-library test (`tests/vst3_host.rs`).
- [x] **Demucs Catalog & MCP Separation:** Registered MIT `demucs-4stem` in `alp-voice::model::standard_catalogue()`, added HTDemucs packed overlap-add in `alp-infer::demucs`, and exposed `dubbing_separate_stems` (#87) via `alp-api -> alp_voice::InferClient` (weights download on demand rather than bundling).
- [x] **Ubuntu 22.04 Compatibility:** Pinned `ort = "=2.0.0-rc.10"` (ORT 1.22, no `GLIBC_2.38+` symbols) and restored `ubuntu-22.04` builds + Ubuntu 22.04 smoke checks in `.github/workflows/release.yml`.
- [ ] **1.0 Promotion (`v1.0.0-rc.1` / `v1.0.0`):** Workspace manifests remain on `v0.1.0-beta.4` prerelease pending explicit user authorization for the `1.0` version bump and release tag.
