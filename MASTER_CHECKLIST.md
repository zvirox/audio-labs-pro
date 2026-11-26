# Audio Labs Pro — Master Verification & Engineering Checklist

> Last verified against codebase: `v0.1.0-beta.3` (12 Rust crates, `frontend/` Tauri v2 + React 19 Linear UI, `packaging/nfpm.yaml`).
> **UI Policy:** The **Linear UI** (shared Tessera-sampled matte palette `#1C1C1C` dark / `#F8F9FA` light, **Magenta** primary accent `#C026D3`/`#A21CAF`, zero green/teal, chart-free, self-hosted Inter, `check:linear` enforced) is **final and locked** per [DESIGN.md](DESIGN.md). No further UI style redesigns are planned.

---

## Executive Summary

| Track | Status | Verified Coverage |
|---|---|---|
| Core Project Model & Block Store (`alp-core`) | **Complete** | Non-destructive clips, fades, gain/pan envelopes, markers, block-store WAL, undo/redo commands |
| Media Codecs, Peaks & Export (`alp-media`) | **Complete** | WAV/FLAC/OGG/MP3 decoding, multi-res peak pyramid, resampling, TPDF dither, loudness export |
| Real-Time Audio Engine (`alp-engine`) | **Complete** | cpal/PipeWire/ALSA I/O, lock-free mixer, transport, scene state, offline renderer |
| DSP & Spectral Engine (`alp-dsp`) | **Core Complete** | Parametric EQ, biquads, compressor/gate/de-esser, true-peak limiter, LUFS meter, RNNoise + spectral denoiser (`denoise.rs`), STFT spectrogram & 2D spectral selection (`spectral.rs`) |
| Voice & Neural Inference (`alp-voice`, `alp-infer`) | **Core Complete** | Kokoro/Kitten/StyleTTS2, Qwen3-TTS cloning + mandatory consent attestation, Moonshine STT, VAD, pronunciation lexicon, GGUF/ONNX download & SHA-256 verification |
| Audio Plugins (`alp-plugin`) | **CLAP + LV2 Complete** | System scanner (`~/.clap`, `/usr/lib/clap`, LV2 bundles), sandboxing, parameter presets |
| Session Bridge & AI Control (`alp-session`, `alp-mcp`, `alp-cli`) | **Complete** | Tauri IPC commands, 68 MCP tools across Observe/Edit/Export tiers, Starship Control bridge (`alp.sock`), live UNIX socket host |
| Desktop Shell & Locked Linear UI (`alp-desktop`, `frontend/`) | **Complete & Locked** | Tauri v2 frameless window (`1280x800`, min `1024x640`, opaque solid surface), TitleBar/Sidebar/Breadcrumb shell, 9 workspaces, Multitrack + Spectrogram canvases, chart-free engine telemetry, `check:linear` gate |
| Packaging & Release (`packaging/nfpm.yaml`, `.github/workflows/`) | **Complete** | Multi-distro `.deb`, `.rpm`, and Arch `.pkg.tar.zst` via `nfpm`; CI + `v0.1.0-beta.1` release pipeline |

---

## Completed Foundation & Desktop Tracks (Phases 0–13, M0–M8)

- [x] **Phase 0–1 (Core, Engine & Media):** 12-crate Cargo workspace (`alp-core`, `alp-media`, `alp-dsp`, `alp-plugin`, `alp-engine`, `alp-voice`, `alp-infer`, `alp-api`, `alp-mcp`, `alp-session`, `alp-desktop`, `alp-cli`), non-destructive `.alpproj` project persistence, 64 KB sample blocks, lock-free ring buffers, offline master/stem renderers, CI dependency/doc/length guards.
- [x] **Phase 2–3 (Capture, Editing & Timeline Foundation):** Live mic capture & VU metering, multi-resolution peak pyramids, selection/split/trim/slip/nudge/fade commands, full undo/redo command stack.
- [x] **Phase 4 (Multitrack Arranger & Mixer):** Multitrack track lanes, gain/pan faders, mute/solo/arm, bus routing, interactive HTML5 Canvas 2D timeline (`MultitrackCanvas.tsx`), playhead scrubbing, magnetic snapping, ripple edit mode, gain/pan clip envelopes, cue markers.
- [x] **Phase 4.7 & Phase 13 (Spectrogram & Multi-View):** `alp-dsp::spectral` STFT FFT engine, per-track `Waveform | Spectrogram | Dual` view modes, calibrated heatmap palettes (Magma, Inferno, Viridis), interactive 2D bounding-box spectral selection with `fc` / `Q` readouts (`SpectrogramCanvas.tsx`).
- [x] **Phase 5 (DSP Chain & Voice Cleanup):** 10-band EQ (`eq.rs`), biquad filters, dynamics compressor/limiter, EBU R128 / ITU-R BS.1770 LUFS metering (`loudness.rs`), voice chain presets (`voice.rs`), `RnnoiseDenoiser`, `SpectralDenoiser`, and `NoiseReduction` (`denoise.rs`).
- [x] **Phase 6–8 (Models, STT & Voice Cloning):** `alp-infer` runners (`kokoro`, `kitten`, `style_tts2`, `qwen3`, `moonshine`, `gguf`), hardware probe, resumable verified downloads, voice cloning consent attestation gate, multi-speaker audiobook script model & TTS stitching.
- [x] **Phase 10 (MCP Host & Starship Bridge):** 68 typed tools (`tool_defs.rs`), Observe / Edit / Export permission tiers, token auth, UI attach approval flow, audit JSONL log, `starship.rs` Unix socket plugin registration (`$XDG_RUNTIME_DIR/starship/bridges/alp.sock`).
- [x] **Phase 11 (Plugin Hosting):** CLAP and LV2 plugin discovery and real-time host integration in `alp-plugin`.
- [x] **M0–M8 (Tauri v2 Migration & Legacy Removal):** Full cutover from `iced` (`alp-ui` / `alp-ui-kit` removed) to Tauri v2 + React 19 + TypeScript; unified Linux packaging via `packaging/nfpm.yaml` (`.deb`, `.rpm`, `.pkg.tar.zst`).
- [x] **Phase 14 (Spectral Repair & Insert Racks):** `alp-dsp::repair` notch/harmonic STFT effects, `alp-engine::insert_rack` with 8 lock-free controlled slots, and `EffectsWorkspace` native magnitude/phase curve from real `Coefficients::response` telemetry.
- [x] **Linear UI Finalization & Lock:**
  - [x] `TitleBar` (40 px, app mark + version from `tauri.conf.json`, sole `ThemeToggle` storing `ui-theme`, clean drag region, minimize/maximize/`#E5484D` close).
  - [x] Collapsible `Sidebar` (240 px / 56 px, 36 px items, workspace switcher), `Breadcrumb`, `SubtabTopBar`, `InspectorDrawer`, `DockedPlayer`, `CommandPalette` (cmdk), `ShortcutsModal`.
  - [x] Shared matte palette tokens (`styles/linear-theme.css` + Magenta `styles/app-accent.css`, `#1C1C1C`/`#292929` dark, `#F8F9FA`/`#FFFFFF` light, zero green/teal), self-hosted Inter font, `tabular-nums` stats/tables, flat discrete segmented `Meter` strips (`--sky` -> `--amber` -> `--red`).
  - [x] Zero compositor blur (`glass.rs` forces opaque solid surface; KWin X11 blur removed), zero translucent fills, zero glow/gradients, zero dark shadows, verified by `npm run check:linear`.

---

## Remaining Work (Actually Pending Features)

### Production Workflows (Audiobooks, Video Dubbing & External Plugins)
- [ ] Native M4B audiobook packaging muxer (`chpl` chapter atoms + cover art + ID3 tags) in `alp-media`.
- [ ] ACX automated batch compliance export pipeline (automated `-23` to `-18` RMS, `-3 dB` true peak, `-60 dB` noise floor audit report per chapter).
- [ ] `DemucsRunner` neural stem separation (`alp-infer`) splitting vocals/drums/bass/other into multitrack stems.
- [ ] WSOLA speech-rate fitting (`FitPlanner` in `alp-voice`) & video demux/remux pipeline (`VideoRemuxer` in `alp-media`).
- [ ] VST3 plugin host backend (`alp-plugin/src/vst3`) and native plugin editor GUI window embedding.
- [ ] Batch `MacroChain` directory queue runner + `dsp_macro_execute` MCP tool.

### Verification, Soak & QA
- [ ] Automated 25-scenario MCP evaluation benchmark suite with full rollback verification (`mcp_eval.py`).
- [ ] 24-hour continuous multitrack playback soak test and decoder/project parser fuzzing targets.
