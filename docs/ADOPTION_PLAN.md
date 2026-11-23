# Audio Labs Pro — Comprehensive Adoption & Evolution Plan

> **Document Code:** `ALP-PLAN-ADOPTION-2026`  
> **Source Analysis:** [REAVSA.md](file:///home/zvirox/Projects/audio-labs-pro/REAVSA.md)  
> **Target Scope:** Audio Labs Pro (`alp`) Core Platform & Workspaces  
> **Release Cadence:** Weekly Alpha Milestones (`alpha.7`–`alpha.10`) ➔ Bi-Weekly Beta Releases (`beta.1`–`beta.3`) ➔ `v1.0.0` General Availability

---

## 1. Executive Strategy & Evolution Vision

Following the comprehensive reverse-engineering analysis in `REAVSA.md`, Audio Labs Pro is evolving to bridge the gap between two traditionally separate paradigms:
1. **Neural Voice Synthesis & Cloning Platforms:** Advanced text-to-speech, zero-shot voice cloning, prompt-driven voice design, audiobook compilation, and speech-to-speech dubbing.
2. **Professional Desktop Multitrack Audio Editors / DAWs:** Multi-channel timeline mixing, non-destructive clip editing, volume and pan automation envelopes, dual-view spectrograms, surgical 2D spectral notch/repair, and VST3/CLAP/LV2 plugin hosting.

### The UI Architecture Transformation: Dual-Axis Navigation
To resolve the primary ergonomic bottleneck of dual-vertical-sidebar layouts (which steal 300px+ of continuous horizontal screen space), Audio Labs Pro adopts the **Dual-Axis Navigation System**:
- **Vertical Primary Rail (Left, 56px):** Persistent, compact launcher for high-level studio domains (`Voice Lab`, `Multitrack Studio`, `Audiobook Studio`, `Dubbing Studio`, `Audio Tools`, `Effects & Plugins`, `Settings`).
- **Horizontal Subtab Top Bar (Top, 40px):** Replaces the secondary vertical sidebar entirely. Positioned directly beneath the window header, offering instantaneous sub-view toggling while liberating 100% of horizontal screen width for waveforms, spectral editing, and multitrack DAW arrangements.
- **Collapsible Right Inspector (260px, `Ctrl+B`):** Dedicated to fine-grained parameter tweaks, dockable and hideable on demand.

---

## 2. Master Release Timeline & Milestones

| Version | Phase | Milestone Domain | Target Deliverables | Target Release Date | Status |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **`v0.0.1-alpha.6`** | Phase 11b | Parity Baseline | Initial voice profile lock/update, batch TTS queue, audiobook encode parity | **Nov 23, 2026** | **Shipped** |
| **`v0.0.1-alpha.7`** | **Phase 12** | Dual-Axis Shell & Multitrack Core | Dual-Axis Navigation shell (Rail + Subtabs), multitrack clip model, envelope curves, SQLite block storage schema | **Dec 07, 2026** | **In Progress** |
| **`v0.0.1-alpha.8-m`** | **Phase M0–M8** | **UI Migration (iced → Tauri 2 + React)** | M0 feasibility gate, M1 alp-session extraction, M2 Tauri scaffold, M3 kit/shell, M4 transport/waveform, M5 workspace ports, M6 multitrack canvas, M7 AI/MCP parity, M8 packaging cutover | **Dec 2026 – Jan 2027** | **Proposed** |
| **`v0.0.1-alpha.8`** | **Phase 13** | Multitrack Canvas & Spectral Multi-View | Interactive DAW canvas (Canvas 2D/WebGL in React), clip split/slip/ripple, simultaneous Waveform + STFT Spectrogram, 2D spectral selection | **Dec 14, 2026** | **Planned** |
| **`v0.0.1-alpha.9`** | **Phase 14** | Advanced Spectral DSP & FX Racks | Subtractive spectral denoiser with profile sampling, spectral notch/repair, realtime insert FX racks, WSOLA time stretch | **Dec 21, 2026** | **Planned** |
| **`v0.0.1-alpha.10`** | **Phase 15** | Voice Lab Expansion & Voice Design | Zero-shot cloning embeddings, prompt-driven voice design, persona cards & avatars, pronunciation lexicon with IPA/SSML | **Dec 28, 2026** | **Planned** |
| **`v0.1.0-beta.1`** | **Phase 16** | Audiobook Studio & Long-Form Engine | Manuscript ingestion (EPUB/MD), multi-character cast tagging, cadence pacing rules, M4B containerization with chapters | **Jan 08, 2027** | **Planned** |
| **`v0.1.0-beta.2`** | **Phase 17** | Video Dubbing & Stem Separation | Local Demucs v4 vocal/stem isolation, sync-lock label tracks, speech-to-speech dubbing with rate fitting, FFmpeg video remux | **Jan 18, 2027** | **Planned** |
| **`v0.1.0-beta.3`** | **Phase 18** | VST3 Hosting & Macro Automation | VST3 plugin hosting engine with native window embedding, DSP macro sequencing chains, batch processing queue | **Jan 28, 2027** | **Planned** |
| **`v1.0.0-rc.1`** | **Phase 19** | MCP Agent Ecosystem & Hardening | Complete MCP tool coverage for all studio functions, long-running stability soaks, packaging suite (AppImage/Flatpak/deb/rpm) | **Feb 15, 2027** | **Planned** |

---

## 3. Phase-by-Phase Execution Plan & Achievement Checklists

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│                             Phased Adoption Flow                                 │
├─────────────────────┬─────────────────────┬──────────────────────────────────────┤
│ Phase 12 (alpha.7)  │ Phase M0–M8 (Prop.) │ Phase 13 (alpha.8)                   │
│ Dual-Axis Shell &   │ UI Migration to     │ Multitrack Canvas &                  │
│ Core Clip Model     │ Tauri 2 + React     │ Spectrogram View (React)             │
├─────────────────────┼─────────────────────┼──────────────────────────────────────┤
│ Phase 14 (alpha.9)  │ Phase 15 (alpha.10) │ Phase 16 (beta.1)                    │
│ Spectral DSP &      │ Voice Cloning &     │ Audiobook Studio &                   │
│ Realtime FX Racks   │ Prompt Voice Design │ M4B Packaging                        │
├─────────────────────┴─────────────────────┴──────────────────────────────────────┤
│ Phase 17 (beta.2) ➔ Video Dubbing, Stem Separation & Sync-Lock Labels            │
│ Phase 18 (beta.3) ➔ VST3 Plugin Hosting & Batch Macro Chains                     │
│ Phase 19 (v1.0.0) ➔ MCP Agent Full Coverage, Hardening & Multi-Distro Packaging  │
└──────────────────────────────────────────────────────────────────────────────────┘
```

---

### Phase 12: Dual-Axis Navigation Shell & Multitrack Core Model (`v0.0.1-alpha.7`)
**Target Release Date:** December 07, 2026  
**Primary Crates:** `alp-ui-kit`, `alp-ui`, `alp-core`, `alp-engine`

#### Objectives
1. Implement the **Dual-Axis Navigation System** in `alp-ui` and `alp-ui-kit` (56px left vertical rail + 40px top horizontal subtab bar).
2. Migrate existing 7 workspaces to the new top-level rail and configure context subtabs for each studio domain.
3. Establish the core multitrack data model in `alp-core`: arbitrary audio tracks, non-destructive clips, and volume/pan automation envelopes.
4. Design the transactional SQLite WAL block sample store schema.

#### Achievement Checklist
- [ ] **Dual-Axis Shell UI Components (`alp-ui-kit`):**
  - [ ] Implement `PrimaryRail` component with active indicator, tooltips, and keyboard shortcuts (`Alt+1`..`Alt+7`).
  - [ ] Implement `SubtabTopBar` component with styled pill buttons, active state transitions, and overflow scrolling.
  - [ ] Create `InspectorDrawer` collapsible right sidebar (`Ctrl+B`) with smooth width transition.
- [ ] **Navigation Dispatcher Refactoring (`alp-ui`):**
  - [ ] Refactor `crates/alp-ui/src/app/workspace_layout.rs` to render the `PrimaryRail` on the left and dispatch the `SubtabTopBar` dynamically based on selected workspace.
  - [ ] Define `SubtabId` enums for Voice Lab, Multitrack Studio, Audiobook Studio, Dubbing Studio, Audio Tools, Effects, and Settings.
- [ ] **Multitrack Core Data Model (`alp-core`):**
  - [ ] Implement `AudioClip` struct in `crates/alp-core/src/project/clip.rs`: UUID, timeline offset ($t_{\text{start}}$), source media offset, duration, gain, pan, mute, solo, and fade in/out curves.
  - [ ] Implement `Envelope` struct in `crates/alp-core/src/project/envelope.rs`: sorted control points $(t, v)$, interpolation modes (`Linear`, `Bezier`, `Exponential`), and `evaluate_at(frame: u64) -> f32`.
  - [ ] Implement `TrackKind` supporting `AudioTrack`, `LabelTrack`, and `TimeTrack`.
- [ ] **Block-Based Sample Storage Engine (`alp-core`):**
  - [ ] Implement `BlockStore` in `crates/alp-core/src/storage/block_store.rs`: SQLite WAL backend managing 64KB audio sample blocks.
  - [ ] Add unit tests verifying non-destructive clip splitting without copying raw audio sample blocks.

---

### Phase M0–M8: UI Migration from iced to Tauri 2 + React (`Proposed`)
**Target Release Window:** December 2026 – January 2027  
**Primary Crates / Directories:** `alp-session` (planned), `alp-desktop` (planned), `frontend/` (planned)  
**Reference Architecture:** [`docs/architecture/UI_MIGRATION.md`](architecture/UI_MIGRATION.md), [`docs/adr/0013-ui-migration-tauri.md`](adr/0013-ui-migration-tauri.md)

#### Objectives
1. Replace the iced 0.14 desktop shell with a Tauri 2 shell and a React + TypeScript + Tailwind CSS frontend to attain commercial-grade visual polish (Voicebox/VoiceStudio class).
2. Decouple non-rendering session state, commands, worker queues, and socket hosting into headless `alp-session`.
3. Preserve all engine, core, DSP, media, voice, inference, plugin, and MCP behaviors without breakage.
4. Establish a strict IPC contract and complete parity across all 7 workspaces before cutover.

#### Migration Milestones & Exit Criteria
- [ ] **M0 — Feasibility Spike (Size: S, Gate):**
  - Throwaway Tauri 2 transparent undecorated window; probe KWin blur via `x11rb` and WebKitGTK `backdrop-filter`.
  - Canvas 2D waveform autoscroll of 1-hour file.
  - *Exit Criteria:* Waveform autoscroll $\ge 55$ fps, idle CPU $< 1\%$, compositor blur verified or tint fallback confirmed legible.
- [ ] **M1 — Extract `alp-session` (Size: M):**
  - Extract non-rendering session state, command stack, worker queue, autosave, and `alp-api::control` socket host into `alp-session` (no iced, no tauri).
  - Split `app.rs` before hitting the 1,500-line ceiling.
  - *Exit Criteria:* All 144+ workspace tests pass; existing iced app behaves identically; MCP attach works.
- [ ] **M2 — Tauri Shell & Frontend Scaffold (Size: M):**
  - Create `alp-desktop` and `frontend/` (Vite, React, TypeScript strict, Tailwind CSS).
  - Port design tokens (neon hues, dark/light glass palettes, contrast rules). Generate Rust $\leftrightarrow$ TS types.
  - Audit `unsafe_code = "deny"` against Tauri macros; document any narrowly scoped exception.
  - *Exit Criteria:* Window opens with iced-equivalent background; design token page renders; CI builds both apps.
- [ ] **M3 — Component Kit & App Shell (Size: M):**
  - Implement `frontend/src/kit/` (buttons, toggles, sliders, inputs, tabs, dialogs, tooltips, menus, meters, cards).
  - Implement primary navigation rail with 7 workspaces (`Ctrl+1`..`8`), floating panels, command palette (`Ctrl+K`), shortcuts (`F1`), theme cycle (`Ctrl+D`), Reduce motion (`Ctrl+Shift+M`).
  - *Exit Criteria:* Shell navigation, panel layouts, and `workspace.json` persistence match existing app.
- [ ] **M4 — Transport, Meters & Waveform (Size: M):**
  - Docked player connected to audio engine through `alp-session`.
  - Meters over batched `Channel` (~30 Hz); playhead interpolated in `requestAnimationFrame`.
  - Canvas 2D waveform component fed by binary peak pyramid IPC responses.
  - *Exit Criteria:* 1-hour waveform autoscroll meets M0 gate in app; 10-minute playback with 0 dropped audio callbacks.
- [ ] **M5 — Workspace Ports (Size: L):**
  - Sequentially port workspaces behind feature flags: Settings $\to$ Models $\to$ Voices (with cloning consent gate) $\to$ Generate $\to$ Effects (hosted CLAP/LV2) $\to$ Captures $\to$ Stories.
  - *Exit Criteria:* Screenshot parity review, keyboard navigation parity, and docs page per workspace.
- [ ] **M6 — Multitrack Timeline Editor (Size: L):**
  - Canvas/WebGL timeline in `frontend/src/views/Multitrack/`: clips, fades, automation envelopes, markers, selection.
  - Every edit dispatches an IPC command mapping to an undoable `alp-core::Command`.
  - *Exit Criteria:* All editing shortcuts from `docs/KEYBOARD_SHORTCUTS.md` pass; undo/redo restores identical clip IDs; null tests pass.
- [ ] **M7 — AI & MCP Parity (Size: M):**
  - Frontend subscribes to session events; attached MCP edits appear live and commit as single undo entries.
  - *Exit Criteria:* All `alp-mcp --attach` workflows pass without regression.
- [ ] **M8 — Packaging, Cutover & iced Deletion (Size: M):**
  - Update packaging: `.deb` and `.rpm` runtime dependencies (WebKitGTK); update Flatpak manifest to GNOME runtime platform.
  - Remove `alp-ui`, `alp-ui-kit`, and iced/lilt dependencies; replace `check-ui.sh` with ESLint kit rule.
  - *Exit Criteria:* Full CI test suite passes; tagged alpha release passes 30-min soak, recovery, and null tests.

---

### Phase 13: Interactive Multitrack Canvas & Spectral Multi-View (`v0.0.1-alpha.8`)
**Target Release Date:** December 14, 2026  
**Primary Crates / Modules:** `frontend/` (Canvas 2D / WebGL), `alp-session`, `alp-dsp`, `alp-engine`

#### Objectives
1. Build an interactive, hardware-accelerated multitrack timeline canvas supporting clip manipulation (split, slip, ripple edit) in the React frontend.
2. Render draggable volume and pan automation envelopes overlaid on audio clips.
3. Implement simultaneous Waveform and STFT Spectrogram multi-view with linear and logarithmic frequency scaling.
4. Introduce the 2D time-frequency spectral selection tool.

#### Achievement Checklist
- [ ] **Interactive Multitrack Canvas (`frontend/` & `alp-session`):**
  - [ ] Implement multitrack canvas component in `frontend/src/views/Multitrack/Canvas.tsx` using HTML5 Canvas 2D / WebGL.
  - [ ] Implement timeline ruler with dynamic timecode ticks (`00:00:00.000`) and playhead scrubbing with `requestAnimationFrame` interpolation.
  - [ ] Add clip drag-and-drop, boundary trimming handles, and clip split at playhead (`S` key) dispatching IPC commands to `alp-session`.
  - [ ] Implement ripple editing toggle: cutting or moving a clip automatically shifts subsequent clips in `alp-session`.
- [ ] **Interactive Clip Envelopes (`frontend/`):**
  - [ ] Render volume (dB) and pan ($[-1.0, 1.0]$) curves directly over clip waveforms on canvas.
  - [ ] Add interactive control point creation (double-click), dragging, and curve tension adjustment.
- [ ] **High-Resolution Spectrogram Computation (`alp-dsp`):**
  - [ ] Implement `SpectrogramEngine` in `crates/alp-dsp/src/spectral/spectrogram.rs` using `realfft`.
  - [ ] Support FFT window sizes: 512, 1024, 2048, 4096, 8192 samples with Hanning and Blackman-Harris windowing.
  - [ ] Implement power spectral density normalization and log/linear frequency bin mapping.
- [ ] **Spectrogram Canvas & Spectral Selection (`frontend/` & `alp-dsp`):**
  - [ ] Implement heatmap rendering in `frontend/src/views/Multitrack/SpectrogramCanvas.tsx` (Magma, Inferno, and Viridis palettes).
  - [ ] Implement 2D bounding-box selection tool: select both time span $[t_1, t_2]$ and frequency band $[f_1, f_2]$ with center frequency $f_c$ and bandwidth $Q$ readouts.

---

### Phase 14: Advanced Spectral DSP & Realtime FX Insert Racks (`v0.0.1-alpha.9`)
**Target Release Date:** December 21, 2026  
**Primary Crates / Modules:** `alp-dsp`, `alp-engine`, `alp-session`, `frontend/`

#### Objectives
1. Implement classical spectral subtraction noise reduction with ambient noise profile capture.
2. Build surgical 2D spectral repair algorithms (spectral notch filter and harmonic suppression).
3. Implement non-destructive per-track real-time insert FX chains in `alp-engine`.
4. Implement high-fidelity WSOLA time-stretching and formant-preserving pitch-shifting.

#### Achievement Checklist
- [ ] **Noise Profile Subtraction Engine (`alp-dsp`):**
  - [ ] Implement `NoiseProfile` struct capturing spectral magnitude statistics over selected silence regions.
  - [ ] Implement `SpectralSubtractor` in `crates/alp-dsp/src/denoise/spectral_subtraction.rs` with over-subtraction factor $\alpha$, spectral floor $\beta$, and frequency smoothing.
- [ ] **2D Spectral Repair Tools (`alp-dsp`):**
  - [ ] Implement `SpectralNotchFilter`: Attenuate isolated narrow-band tones (mains hum, whistling) within the selected frequency box.
  - [ ] Implement `HarmonicSuppressor`: Automatically calculate and attenuate harmonic overtones ($2f_c, 3f_c, \dots, nf_c$).
- [ ] **Real-Time Track Insert FX Racks (`alp-engine` & `frontend/`):**
  - [ ] Implement `InsertRack` in `crates/alp-engine/src/insert_rack.rs` supporting up to 8 non-destructive effects per track.
  - [ ] Integrate lock-free ring-buffer parameter updates from UI IPC to the audio thread via `alp-session`.
  - [ ] Implement insert rack UI controls in `frontend/src/kit/` and master bus strip with true-peak brickwall limiter and LUFS meter.
- [ ] **High-Fidelity WSOLA Engine (`alp-dsp`):**
  - [ ] Implement Waveform Similarity Overlap-Add (WSOLA) algorithm in `crates/alp-dsp/src/time_stretch.rs` for tempo adjustment (-50% to +100%) without pitch distortion.
  - [ ] Add independent pitch shifting without tempo alteration.

---

### Phase 15: Voice Lab Expansion & Natural Language Voice Design (`v0.0.1-alpha.10`)
**Target Release Date:** December 28, 2026  
**Primary Crates / Modules:** `alp-voice`, `alp-infer`, `alp-session`, `frontend/`

#### Objectives
1. Implement zero-shot voice cloning with speaker embedding extraction and acoustic similarity verification.
2. Build natural language prompt-driven voice design (prompt -> acoustic parameter mapping).
3. Expand persona management with visual avatar support, style presets, and archetype library.
4. Implement custom pronunciation lexicon with IPA/CMUDict overrides and SSML expansion.

#### Achievement Checklist
- [ ] **Zero-Shot Voice Cloning Pipeline (`alp-voice` & `alp-infer`):**
  - [ ] Integrate ECAPA-TDNN / CAMPPlus speaker embedding extraction in `crates/alp-infer/src/models/speaker_encoder.rs`.
  - [ ] Implement `VoiceCloneEnrollment`: Ingest 3–15 second clean reference WAV, extract 512-dim embedding, and generate `.alpvoice` profile.
  - [ ] Implement `AcousticSimilarityMatrix`: Calculate cosine similarity score ($0.0 \to 1.0$) between reference audio and synthesized takes.
- [ ] **Natural Language Voice Design Engine (`alp-voice` & `frontend/`):**
  - [ ] Implement `VoiceDesignParser` in `crates/alp-voice/src/voice_design.rs`: Map descriptive text prompts to pitch baseline, pitch variance, speech rate, spectral tilt, and breathiness.
  - [ ] Provide interactive acoustic radar chart component in `frontend/src/views/VoiceLab/` showing extracted acoustic traits.
- [ ] **Persona Library & Archetypes (`frontend/` & `alp-voice`):**
  - [ ] Implement `PersonaCard` component in `frontend/src/views/VoiceLab/PersonaCard.tsx` with avatar image display, tag badges, and quick-audition button.
  - [ ] Bundle default voice archetypes across commercial, narration, gaming, and character acting domains.
- [ ] **Pronunciation Lexicon & SSML (`alp-voice`):**
  - [ ] Implement `LexiconManager` in `crates/alp-voice/src/lexicon.rs` supporting user-defined pronunciation overrides (word -> IPA/CMUDict).
  - [ ] Implement SSML tag parser supporting `<break time="...">`, `<emphasis>`, `<phoneme>`, and pitch/rate attributes.

---

### Phase 16: Audiobook Studio & Long-Form Production (`v0.1.0-beta.1`)
**Target Release Date:** January 08, 2027  
**Primary Crates / Modules:** `alp-core`, `alp-voice`, `alp-media`, `alp-session`, `frontend/`

#### Objectives
1. Implement manuscript ingestion supporting EPUB, Markdown, and plain text with automatic chapter and paragraph hierarchy parsing.
2. Build multi-character voice casting engine with heuristic and LLM-assisted speaker dialogue attribution.
3. Implement automated cadence pacing rules (inter-sentence, paragraph, and chapter break silences).
4. Build M4B audiobook containerization with embedded QuickTime chapter markers (`chpl`) and cover artwork.

#### Achievement Checklist
- [ ] **Manuscript Ingestion & Parsing (`alp-core`):**
  - [ ] Implement `ManuscriptParser` in `crates/alp-core/src/project/audiobook.rs` parsing EPUB, Markdown, and TXT files.
  - [ ] Extract hierarchical document model: Book Title, Author, Chapters, Sections, Paragraphs, and Dialogue lines.
- [ ] **Multi-Character Casting Engine (`alp-voice`):**
  - [ ] Implement character speaker attribution tagging: Detect narration vs. character dialogue in quotes.
  - [ ] Allow assigning specific `VoiceProfile` personas to individual characters.
- [ ] **Cadence & Pacing Automation (`alp-voice`):**
  - [ ] Implement configurable pause insertion: Comma (150ms), Period (400ms), Paragraph break (900ms), Scene transition (2000ms), Chapter break (3500ms).
- [ ] **M4B Container & Chapter Packaging (`alp-media`):**
  - [ ] Implement `AudiobookPacker` in `crates/alp-media/src/audiobook_packer.rs`: Concatenate chapter audio into seamless AAC/M4A streams.
  - [ ] Generate QuickTime chapter marker atoms (`chpl`) with chapter titles and exact millisecond timecodes.
  - [ ] Embed high-resolution cover art (JPEG/PNG) and ID3 tags (Narrator, Series, Release Year).
- [ ] **Audiobook Studio Workspace (`frontend/`):**
  - [ ] Build Audiobook Studio UI in `frontend/src/views/AudiobookStudio/` with horizontal subtabs: `[ Manuscript ]`, `[ Chapters ]`, `[ Character Cast ]`, `[ Cadence Rules ]`, `[ M4B Export ]`.

---

### Phase 17: Video Dubbing, Stem Separation & Sync-Lock Labels (`v0.1.0-beta.2`)
**Target Release Date:** January 18, 2027  
**Primary Crates / Modules:** `alp-infer`, `alp-core`, `alp-media`, `alp-dsp`, `alp-session`, `frontend/`

#### Objectives
1. Integrate local neural stem separation (Demucs v4 4-stem model via ONNX).
2. Implement sync-locked label tracks synchronized with audio editing actions.
3. Build end-to-end speech-to-speech dubbing pipeline with directorial speech-rate fitting.
4. Implement lossless video demuxing and remuxing via FFmpeg.

#### Achievement Checklist
- [ ] **Neural Stem Separation Engine (`alp-infer` & `alp-dsp`):**
  - [ ] Implement `DemucsRunner` in `crates/alp-infer/src/models/demucs.rs` executing quantized ONNX Demucs v4.
  - [ ] Split input audio files into 4 isolated stems: `vocals.wav`, `drums.wav`, `bass.wav`, `other.wav`.
  - [ ] Auto-populate isolated stems into separate tracks in the Multitrack timeline.
- [ ] **Sync-Locked Label Tracks (`alp-core` & `frontend/`):**
  - [ ] Implement `LabelTrack` in `crates/alp-core/src/project/track.rs` supporting point labels and region labels.
  - [ ] Implement sync-lock logic: When sync-lock is enabled, time edits (insert, delete, cut, ripple) propagate to label tracks.
  - [ ] Add bidirectional SRT, WebVTT, and TSV import and export.
- [ ] **Speech-to-Speech Dubbing & Speech-Rate Fitting (`alp-voice` & `alp-dsp`):**
  - [ ] Implement automated pipeline: Vocal separation ➔ Whisper transcription ➔ Translation ➔ Speaker-matched synthesis.
  - [ ] Implement `FitPlanner` in `crates/alp-voice/src/rate_fitting.rs`: Calculate time-stretch ratio $\rho = T_{\text{target}} / T_{\text{synth}}$ and apply WSOLA to lock speech takes to video timecodes.
- [ ] **Video Demux / Remux Pipeline (`alp-media`):**
  - [ ] Implement `VideoRemuxer` in `crates/alp-media/src/video_sync.rs`: Extract audio from MP4/MKV and remux newly dubbed master track with `-c:v copy`.
- [ ] **Dubbing Studio Workspace (`frontend/`):**
  - [ ] Build Dubbing Studio UI in `frontend/src/views/DubbingStudio/` with horizontal subtabs: `[ Video Timeline ]`, `[ Stem Separation ]`, `[ Transcript & Translate ]`, `[ Rate Fitting ]`, `[ Remux & Export ]`.

---

### Phase 18: VST3 Plugin Hosting & Batch Macro Automation (`v0.1.0-beta.3`)
**Target Release Date:** January 28, 2027  
**Primary Crates / Modules:** `alp-plugin`, `alp-engine`, `alp-desktop`, `frontend/`, `alp-mcp`

#### Objectives
1. Implement full VST3 plugin hosting architecture alongside existing CLAP and LV2 loaders.
2. Build native X11/Wayland plugin GUI window embedding anchored to the desktop window handle.
3. Implement scriptable and UI-driven DSP macro chaining across batch processing queues.

#### Achievement Checklist
- [ ] **VST3 Plugin Host Engine (`alp-plugin`):**
  - [ ] Implement `Vst3Host` in `crates/alp-plugin/src/vst3/mod.rs` with module discovery in `/usr/lib/vst3` and `~/.vst3`.
  - [ ] Instantiate VST3 component processors, query parameter buses, and process 32-bit float audio blocks in realtime audio callback.
- [ ] **Plugin GUI Window Embedding (`alp-plugin` & `alp-desktop`):**
  - [ ] Implement native window parenting bridge in `alp-plugin` / `alp-desktop` for X11 and Wayland (via XWayland) anchored to the Tauri window handle.
  - [ ] Provide floating plugin window controls with parameter preset save/load in the React UI.
- [ ] **DSP Macro Sequencing Engine (`alp-engine` & `alp-mcp`):**
  - [ ] Implement `MacroChain` in `crates/alp-engine/src/macro_chain.rs`: Chainable sequence of DSP processors (e.g., Denoise ➔ EQ ➔ Compressor ➔ Normalize ➔ Export).
  - [ ] Support batch execution over directory queues with multi-threaded worker pools.
  - [ ] Expose macro execution to MCP via `dsp_macro_execute` tool.

---

### Phase 19: Full MCP Autonomous Agent Automation & Hardening (`v1.0.0-rc.1` / `v1.0.0`)
**Target Release Date:** February 15, 2027  
**Primary Crates / Modules:** `alp-mcp`, `alp-api`, `alp-desktop`, `frontend/`, Workspace Crates

#### Objectives
1. Achieve 100% MCP tool coverage across all newly adopted capabilities (voice design, multitrack editing, spectral cleanup, audiobook packaging, video dubbing).
2. Execute comprehensive stress tests, long-running audio dropout soaks, and memory leak audits.
3. Validate and produce multi-distribution release packages (AppImage, Flatpak, `.deb`, `.rpm`) using Tauri 2.

#### Achievement Checklist
- [ ] **Full MCP Tool Coverage (`alp-mcp`):**
  - [ ] Voice Lab tools: `voice_design_prompt`, `voice_clone_enroll`, `voice_similarity_audit`, `voice_persona_create`.
  - [ ] Multitrack tools: `timeline_track_create`, `timeline_clip_insert`, `timeline_envelope_set`, `timeline_clip_split`, `timeline_clip_slip`.
  - [ ] Spectral & DSP tools: `dsp_spectral_denoise`, `dsp_spectral_notch`, `dsp_time_stretch_fit`, `dsp_stem_separate`.
  - [ ] Production tools: `audiobook_build_full`, `dubbing_execute_pipeline`, `macro_batch_run`.
- [ ] **Evaluation & Benchmark Harness (`mcp_eval.py`):**
  - [ ] Add 25 automated evaluation scenarios testing complex multi-step agent operations.
  - [ ] Enforce regression pass rate of 100% on all tool executions with session rollback verification.
- [ ] **Stability, Soak & Fuzzing Audit:**
  - [ ] Execute 24-hour continuous multitrack audio playback soak test (0 dropped frames, constant RSS memory).
  - [ ] Fuzzing of project file parser, block store deserializer, and media container decoders.
- [ ] **Packaging & Release Deployment:**
  - [ ] Update and verify `scripts/build-appimage.sh` for `alp-desktop` + `frontend/`.
  - [ ] Generate and validate Flatpak manifest (`io.github.zvirox.AudioLabsPro.yml`) on GNOME runtime platform.
  - [ ] Build Debian (`.deb`) and Red Hat (`.rpm`) packages with WebKitGTK runtime dependencies and desktop MIME integration.

---

## 4. Verification & Guard Compliance

Every phase implementation must pass all four repository guard scripts and the workspace test suite before landing:

```bash
# 1. Enforce strict 1,500 line limit per Rust source file
bash scripts/check-file-length.sh

# 2. Enforce documentation coverage with all 8 mandatory sections for all modules
bash scripts/check-docs.sh

# 3. Enforce strict acyclic architectural dependency graph
bash scripts/check-deps.sh

# 4. Enforce design system adherence and zero raw iced button widgets
bash scripts/check-ui.sh

# 5. Full workspace unit and integration test suite
cargo test --workspace
```
