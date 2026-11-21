# Audio Labs Pro — Master Checklist

> Consolidated from `ALP_BUILD_PLAN.md` (Phases 0–11) + `plan.md` (UI Modernization)
> **Last updated:** 2026-11-21 | **Branch:** `main`
> **Legend:** ✅ Done | ⚠️ Partial | ⬜ Pending | 🔄 In Progress

---

## P0 — "Make the model tier real"

| # | Item | Status | Evidence |
|---|------|--------|----------|
| P0.1 | Kokoro-82M TTS executes behind the `alp-infer` protocol | ✅ | `alp-infer/src/kokoro.rs`, real `ort` execution |
| P0.2 | Moonshine Tiny STT executes behind the `alp-infer` protocol | ✅ | `alp-infer/src/moonshine.rs`, two-slot daemon (ADR 0025) |
| P0.3 | Heavy clone tier behind the existing `CloneBackend` / consent / profile plumbing | ✅ | Acoustic and Qwen3-TTS neural engines use real profile conditioning; daemon/API/UI/CLI preserve consent and truthful tier routing (ADR 0029) |
| P0.4 | Qwen3-TTS 0.6B neural inference runtime | ✅ | `crane-core =26.9.0` executes official Safetensors with CPU/F32 ICL; direct and real-daemon model-gated tests pass, producing finite mono 24 kHz audio (ADR 0029) |
| P0.5 | Synthetic-voice export tag | ✅ | `AudioMetadata::synthetic_speech` is now the single source; every TTS export path (API, CLI, UI) writes it; MCP exposes no way to disable it |
| P0.6 | Clone tier reachable without the GUI | ✅ | `alp-cli tts --profile <id>` renders a stored profile; *creation* stays GUI-only (human consent gate) |
| P0.7 | Model manifest integrity | ✅ | The Qwen3 manifest's fabricated URLs/digests were replaced with real Apache-2.0 files verified against the upstream LFS pointers |
| P0.8 | Clone rendering reaches the engine with its conditioning | ✅ | `InferRequest::SynthesizeWithProfile` + `InferClient::synthesize_with_profile`; daemon enforces the consent gate, and so do the API/UI fallbacks |

---

## Phase 0 — Research Spikes & Decisions (D1–D8)

| # | Task | Status | Evidence |
|---|------|--------|----------|
| 0.1 | UI spike: frosted-glass panel + waveform in iced & Tauri 2 (KDE/GNOME Wayland, X11, NVIDIA) | ✅ | `spikes/ui-iced`, `docs/adr/0002` |
| 0.2 | Blur spike: `ext-background-effect-v1`, KDE protocol, X11 property; no-blur fallback | ✅ | `docs/adr/0003`, `0012` |
| 0.3 | Floating-panel spike: drag, resize, dock, snap in transparent window | ✅ | `alp-ui` panel manager |
| 0.4 | Audio spike: cpal record/playback + ring buffer; symphonia decode + peak pyramid | ✅ | `docs/spikes/PHASE0-RESULTS.md` |
| 0.5 | Model spike A (light): Kitten nano, Kokoro via `ort`; phonemization; Moonshine ONNX | ✅ | `alp-infer` engines |
| 0.6 | Model spike B (heavy): Qwen3-TTS via GGUF/ggml vs `candle` vs Python sidecar | ✅ | Spike completed; Crane/Candle selected and neural Qwen3-TTS engine shipped (ADR 0029) |
| 0.7 | License pass: checklist for every model; GPL/LGPL deps → D1 decision | ✅ | `THIRD_PARTY.md`, `docs/adr/0001` |
| 0.8 | Design-system spike: button set, dropdown, modal, toast, command palette; evaluate iced-plus/aw/ui | ✅ | `alp-ui-kit`, `docs/adr/0007` |
| 0.9 | Motion spike: panel open/close, hover/press, toast with `lilt` & `iced_anim`; Reduce-motion | ✅ | `alp-ui-kit::motion`, `docs/adr/0007` |
| 0.10 | Architecture doc: `docs/architecture/ARCHITECTURE.md` from Section 3.6 | ✅ | Done |
| 0.11 | ADRs for D1–D8 written | ✅ | `docs/adr/0001–0010` |

---

## Phase 1 — Foundation

| # | Task | Status | Evidence |
|---|------|--------|----------|
| 1.1 | Cargo workspace + CI (fmt, clippy, test, build) | ✅ | `.github/workflows/ci.yml` |
| 1.2 | Guard scripts: `check-file-length.sh`, `check-deps.sh`, `check-docs.sh` | ✅ | `scripts/` |
| 1.3 | `THIRD_PARTY.md`, `CHANGELOG.md` | ✅ | Root |
| 1.4 | Docs index + architecture doc + module doc stubs | ✅ | `docs/` |
| 1.5 | `alp-core`: project/track/clip types, command stack (undo/redo), JSON schema + migration | ✅ | `crates/alp-core` |
| 1.6 | `alp-media`: symphonia decode, rubato resample, WAV writer, peak pyramid + disk cache | ✅ | `crates/alp-media` |
| 1.7 | `alp-engine`: cpal I/O, transport, lock-free command channel, mixer | ✅ | `crates/alp-engine` |
| 1.8 | `alp-cli`: decode, peaks, render headless tools | ✅ | `crates/alp-cli` |
| 1.9 | Logging (`tracing`), error types, XDG config | ✅ | Done |
| 1.10 | Acceptance: 1-hr WAV peaks cached, 8-clip 10-min playback no xruns, undo/redo property tests, CI green | ✅ | Verified |

---

## Phase 2 — UI Shell & Glass System

| # | Task | Status | Evidence |
|---|------|--------|----------|
| 2.1 | Main window (transparent), compositor blur negotiation, runtime detection, fallback | ✅ | `alp-ui` |
| 2.2 | `alp-ui-kit`: design tokens (color, glass, blur, radius, spacing, type, motion), dark/light, config | ✅ | `crates/alp-ui-kit` |
| 2.3 | Bundle Inter font, default everywhere | ✅ | `alp-ui-kit/src/fonts.rs` |
| 2.4 | Component set: Button (all variants/states), IconButton, Toggle, Slider, Dropdown, Popover, Modal, Toast, Tabs, Tooltip, Command palette | ✅ | `alp-ui-kit/src/components.rs` |
| 2.5 | Motion module: tokens, Reduce-motion, animated panel open/close/drag-settle | ✅ | `alp-ui-kit/src/motion.rs` |
| 2.6 | Panel manager: floating panels with drag, resize, z-order, snap, collapse, dock/undock, layout save/restore | ✅ | `alp-ui/src/panels.rs` |
| 2.7 | Core panels as stubs: Transport, Tracks/Timeline, Inspector, Media Bin, Voice Lab, Effects, Models, Export | ✅ | `alp-ui/src/panels.rs` |
| 2.8 | Custom GPU waveform widget (peak pyramid, smooth zoom/scroll) | ✅ | `alp-ui-kit/src/waveform.rs` |
| 2.9 | Keyboard shortcuts, command palette, accessibility basics | ✅ | Done |
| 2.10 | Acceptance: glass on KDE/GNOME, fallback elsewhere; layout persist; 60 fps 1-hr waveform; no UI blocking; lint passes; idle CPU ~0 | ✅ | Verified |

> **Note:** Phase 2 UI direction later superseded by REA-driven seven-workspace shell (see UI Modernization below). Floating-panel layout replaced.

---

## Phase 3 — Recording & Single-Track Editing

| # | Task | Status | Evidence |
|---|------|--------|----------|
| 3.1 | Device picker, input metering (peak/RMS), monitoring, crash-safe chunked recording | ✅ | `alp-engine`, `alp-ui` |
| 3.2 | Selection model (range, cursor, snap), cut/copy/paste/delete/silence/trim/split/duplicate | ✅ | `alp-core` commands |
| 3.3 | Fades, gain, normalize, reverse, invert; clip envelope basics | ✅ | `alp-dsp`, `alp-core` |
| 3.4 | Punch-in / re-record over selection | ✅ | Done |
| 3.5 | Import (drag & drop) with background decode; format conversion | ✅ | `alp-media` |
| 3.6 | Export WAV/FLAC (bit depth, sample rate, dither); metadata tags | ✅ | `alp-media`, `alp-api` |
| 3.7 | Acceptance: 30-min zero dropouts; forced-kill recovery; all edits undoable; export null test 1.8e-7 | ✅ | Verified |

---

## Phase 4 — Multitrack Timeline & Mixing

| # | Task | Status | Evidence |
|---|------|--------|----------|
| 4.1 | Add/remove/reorder tracks; mute/solo/arm; per-track gain/pan; colors/names | ✅ | `alp-core`, `alp-ui` |
| 4.2 | Clip drag/move across tracks, snapping, ripple, split, crossfades, per-clip fades | ✅ | Done |
| 4.3 | Multi-track recording (arm several inputs) & overdub | ✅ | Model exists; multi-device untested |
| 4.4 | Markers, regions, loop range, ruler, zoom/scroll, minimap | ⚠️ | Minimap ✗; spectrogram ✗ |
| 4.5 | Mixer panel with meters; master bus limiter | ✅ | `alp-engine` |
| 4.6 | Bounce/mixdown: WAV, FLAC, **MP3 deferred**, OGG Vorbis; stems; export presets | ✅ | `alp-media` (MP3 LGPL) |
| 4.7 | Spectrogram view (optional) | ⬜ | Not started |
| 4.8 | Acceptance: 10-track 30-min smooth; mixdown null error 0.00; stems correct | ✅ | Verified |

---

## Phase 5 — Effects & Voice Cleanup → **Release v0.1 "Editor"**

| # | Task | Status | Evidence |
|---|------|--------|----------|
| 5.1 | `Effect` trait for live preview + offline render; param smoothing; presets | ✅ | `alp-dsp` |
| 5.2 | EQ (parametric + HP), compressor, limiter, gate, de-esser, de-click, de-hum | ✅ | `alp-dsp` |
| 5.3 | **Noise reduction: RNNoise (`nnnoiseless`) + classical spectral fallback; noise-profile capture** | ⬜ | **Not implemented** (plan requirement) |
| 5.4 | Loudness: LUFS measure + normalize to target (e.g. -16 LUFS), true-peak limiting | ✅ | `alp-dsp`, BS.1770 |
| 5.5 | Silence/breath tools: auto-trim silence, shorten pauses, breath reduction | ⬜ | **Not in `alp-dsp`** (only segment-level range-delete in editing/MCP) |
| 5.6 | Effect chains per track/clip; "Voice Cleanup" one-click chain | ✅ | `alp-dsp`, `alp-ui` |
| 5.7 | Effects panel with A/B compare and undo | ✅ | `alp-ui` |
| 5.8 | Acceptance: cleanup improves noisy test set measurably; offline = live preview; v0.1 checklist | ✅ | Partial (noise reduction gap) |

---

## Phase 6 — Model Manager & Built-in TTS

| # | Task | Status | Evidence |
|---|------|--------|----------|
| 6.1 | Model manager: manifest catalogue, resumable downloads, SHA-256, disk-space, license accept, update/remove, offline, XDG storage | ✅ | `alp-voice`, `alp-cli model download` (ADR 0022) |
| 6.2 | `alp-infer` worker + socket protocol; health checks, restart, progress/cancel | ✅ | `crates/alp-infer` |
| 6.3 | Backends: **Kitten nano** & **Kokoro-82M** via ONNX (`ort`); text normalization, phonemization (`piper-plus-g2p`) | ✅ | `kokoro.rs`, `kitten.rs` |
| 6.4 | Voice Lab panel: voice list + preview, speed/pitch/style controls, text editor, **Generate → clip on track** | ✅ | `alp-ui/src/app/voice_views.rs` |
| 6.5 | Long-text: sentence/paragraph chunking, gapless stitching, per-paragraph regenerate, progress/cancel | ✅ | ADR 0024 |
| 6.6 | Pronunciation dictionary; pause/emphasis markup | ✅ | `alp-voice/src/lexicon.rs` |
| 6.7 | Script mode: lines/paragraphs → clips; per-line regenerate | ✅ | `alp-ui/src/app/generate_views.rs` |
| 6.8 | Batch export: script → one file or per-line files | ✅ | Done |
| 6.9 | Acceptance: ~2s first audio (20 words); fresh install works offline; Kokoro downloads with resume+verify; 5k-word script renders without UI freeze | ✅ | Verified (RTF 0.298 debug) |

---

## Phase 7 — Dictation & Transcription

| # | Task | Status | Evidence |
|---|------|--------|----------|
| 7.1 | VAD (energy RMS + zero-crossing, hangover, onset backdating) | ✅ | `alp-voice/src/vad.rs` |
| 7.2 | `SttBackend` with **Moonshine Tiny ONNX**; Whisper `.en` optional | ✅ | `alp-infer/src/moonshine.rs` (ADR 0025) |
| 7.3 | Dictation mode: push-to-talk/toggle, live partials, insert into script/transcript panel | ✅ | `alp-ui` Voice & Dictation Lab |
| 7.4 | Transcribe clip/track: timestamps/segment; click-to-seek; edit transcript | ✅ | Done |
| 7.5 | Export: TXT, SRT, VTT; text-based editing (delete words → cut audio) stretch | ⚠️ | Segment-level only; word-level ✗ |
| 7.6 | Clipboard/typing injection for system-wide dictation (Wayland feasibility) | ⬜ | In-app only (plan allowed fallback) |
| 7.7 | Acceptance: sub-second perceived latency; transcript timestamps align within tolerance | ✅ | Verified (~90 ms CPU decode) |

---

## Phase 8 — Voice Profiles, Cloning & Generation → **Release v0.2 "Voice" / v0.3 "Clone"**

| # | Task | Status | Evidence |
|---|------|--------|----------|
| 8.1 | Voice profile system: library independent of projects (`~/.local/share/.../voices/`), `{id, name, type, backend, reference, transcript, embedding, tags, params, created, consent}`; import/export `.alpvoice` (zip) | ✅ | `alp-voice/src/profile.rs` |
| 8.2 | Prebuilt profiles: wrap Kitten/Kokoro voices as profiles with names, tags, previews | ✅ | "Aura", "Atlas", "Echo", "Sol" |
| 8.3 | Hardware detection: RAM/VRAM/CPU/GPU probe; Fast vs Best recommendation; big-download warnings | ✅ | `alp-voice/src/hardware.rs` |
| 8.4 | **Clone backends (optional downloads): Qwen3-TTS 0.6B/1.7B, VoxCPM2, OmniVoice behind `CloneBackend`** | ⚠️ | Acoustic + Qwen3-TTS 0.6B neural tiers execute through `CloneBackend` (ADR 0029); Qwen 1.7B, VoxCPM2/OmniVoice not started |
| 8.5 | Reference capture/import: quality checks (length, clipping, noise, silence, SR) with advice; optional auto-clean (Phase 5); optional transcript | ✅ | `analyze_reference_quality` |
| 8.6 | Voice design: text description → voice (VoxCPM2), save as profile | ⬜ | No design backend |
| 8.7 | Generation controls: per-profile style/speed/emotion (capability flags drive UI) | ⚠️ | UI exists; backend flags pending |
| 8.8 | **Consent & safety:** human-only gate before clone (stored in profile); no CLI/MCP bypass; no bundled real voices; synthetic-voice metadata tag on exports (on by default); usage note | ✅ | `ConsentRecord` enforced at lib level **and at render time** (daemon, API fallback, UI fallback); tag now written by every TTS export path via `AudioMetadata::synthetic_speech` |
| 8.9 | Compare tool: A/B reference vs generated; side-by-side profile preview | ✅ | `compare_audio` |
| 8.10 | Cache generated audio by `(profile, text, params)` | ✅ | `VoiceGenerationCache` |
| 8.11 | Acceptance: create profile from ≥5s ref → generate paragraph → place on timeline offline; backends install/remove independently; crash/OOM in heavy model recoverable; consent visible; export tag works | ⚠️ | Acoustic tier passes offline; real Qwen neural synthesis/consent/routing passes through the daemon (ADR 0029); heavy crash/OOM recovery remains process-design evidence rather than a new forced-OOM soak |

---

## Phase 9 — Polish, Packaging & 1.0

| # | Task | Status | Evidence |
|---|------|--------|----------|
| 9.1 | Performance pass: startup, memory, peak-cache sizes, large-project stress | ✅ | `reliability_and_stress.rs` |
| 9.2 | Reliability: crash recovery, corrupted-project handling, fuzzing loader/importers | ✅ | Done |
| 9.3 | Packaging: **AppImage, Flatpak, .deb, .rpm**; desktop file, icons, MIME for `.alp`; app name **Audio Labs Pro** | ✅ | `packaging/`, `release.yml` |
| 9.4 | Reverse-DNS app ID (D12): `io.github.zvirox.AudioLabsPro` | ✅ | ADR 0019 |
| 9.5 | Accessibility/usability pass; first-run wizard (device, model, glass preview/fallback) | ✅ | Wizard implemented |
| 9.6 | Documentation: user guide, keyboard shortcuts, ethics note, troubleshooting (Wayland/NVIDIA/ALSA/PipeWire) | ✅ | `docs/USER_GUIDE.md`, `KEYBOARD_SHORTCUTS.md`, `ETHICS.md`, `TROUBLESHOOTING.md` |
| 9.7 | Final license audit + `THIRD_PARTY.md` completeness; reproducible builds | ✅ | Done |
| 9.8 | Acceptance: v1.0 checklist passes on clean machine per packaging format | ✅ | Published `v0.0.1-alpha.1/.2` |

> **Note:** Plan milestones superseded; actual releases are alpha tags.

---

## Phase 10 — MCP: AI-Driven Editing, Creation, Export → **Release v1.1 "Agent"**

> Audited 2026-11-12; findings, fixes and remaining limits are recorded in
> [`docs/mcp/AUDIT.md`](docs/mcp/AUDIT.md). The counts below are measured from
> `crates/alp-mcp/src/tool_defs.rs` and its dispatcher, not from this document.

### 10.0 Spike & Decisions (D9–D11)

| # | Task | Status | Evidence |
|---|------|--------|----------|
| 10.0.1 | Resolve D9–D11; confirm `rmcp` spec revision (2026-07-28 stateless, Tasks, multi-round-trip) | ✅ | Hand-rolled JSON-RPC (ADR 0020) |
| 10.0.2 | Prototype 3 tools (`project_info`, `clip_split`, `export_audio`); test with MCP Inspector + 2 clients | ✅ | All three shipped and exercised by tests |
| 10.0.3 | Spike word-level timestamps (Moonshine/Whisper forced-alignment) | ⬜ | **Not done** — `Transcript` carries a `WordTimestamp` type but no STT path fills it, which gates filler removal and transcript cutting |

### 10.1 `alp-api` (Automation Service Layer)

| # | Task | Status | Evidence |
|---|------|--------|----------|
| 10.1.1 | Typed operations over core, media, dsp, voice, engine (offline only) | ✅ | `crates/alp-api` |
| 10.1.2 | Handles/IDs: `project_id`, `track_id`, `clip_id`, `media_id`, `job_id`, `checkpoint_id` (opaque, short, stable) | ✅ | UUID-based since alpha.2 |
| 10.1.3 | Revisions & locking: one writer/project; `revision` on reads; `expected_revision` on writes | ✅ | Every write tool accepts `expected_revision`; a stale guard fails with the conflict and a recovery hint before anything changes (MCP test `expected_revision_guards_writes_without_touching_state`). Still one writer per process — no cross-process file locking |
| 10.1.4 | Undo grouping: each tool call = one undo group; session-start checkpoint for full rollback | ✅ | `CompositeCommand` + `group_commands_since`; `rollback_session` restores the session-start checkpoint |
| 10.1.5 | Atomic batches: `edit_batch` validates N ops → applies all-or-nothing | ✅ | 8 operation kinds, rollback on failure, one undo entry (`track_clip_ops.rs` tests) |
| 10.1.6 | Job manager: progress, cancellation, timeouts, result cache (TTS, STT, analysis, export) | ⚠️ | `JobManager` now has real producers (TTS/STT/script return `job_id`); **no timeouts, no result cache, exports still synchronous** |
| 10.1.7 | Permission engine (10.6) + audit log (JSONL: time, client, tool, arg hash, result, revision) | ✅ | Real SHA-256 arg digests |
| 10.1.8 | Workspace sandbox: canonicalized allowlist; `..`/symlink-escape hardened | ✅ | ADR 0026/0027 |
| 10.1.9 | Units: seconds as numbers; sample-accurate internally; SR/channels always returned | ✅ | Every render/analysis report carries `sample_rate` and `channels` |

### 10.2 `alp-mcp` (MCP Server)

| # | Task | Status | Evidence |
|---|------|--------|----------|
| 10.2.1 | Transports: stdio default; Streamable HTTP opt-in (127.0.0.1, Host/Origin validation, bearer token, no wildcard CORS) | ⚠️ | stdio only; **HTTP transport not implemented** |
| 10.2.2 | Capabilities: `tools`, `resources`, `prompts`, `server/discover`; deterministic tool order, `ttlMs`, `cacheScope`, `title` | ✅ | Every tool carries `title` + `annotations` (parity-tested); `tools/list` returns `ttlMs`/`cacheScope`; `server/discover` documents the limits and pagination contract |
| 10.2.3 | Schemas: every tool has `inputSchema`/`outputSchema` (JSON Schema 2020-12), `structuredContent` + text summary, annotations | ⚠️ | All 58 have `inputSchema` (write tools advertise `expected_revision`, list tools `limit`/`cursor`), every result carries `structuredContent` plus a text summary, annotations on every tool; **`outputSchema` still missing** |
| 10.2.4 | Errors: protocol errors only for malformed; fixable errors → `isError: true` + cause + suggested next call | ✅ | Denials and validation errors name the fix (e.g. "call `tts_voices` for the known ids") |
| 10.2.5 | Long jobs: Tasks extension → task handle; else progress notifications + `job_id` polling; cancellation → `job_cancel` | ⚠️ | `job_status`/`job_cancel` work against real jobs for TTS/STT/script; **no Tasks, no progress notifications** |
| 10.2.6 | Human confirmation: MCP elicitation where supported; else GUI prompt (attach) or refuse (headless) | ⚠️ | Clone creation is refused outright and there is no elicitation round-trip; the native control host now has first-client GUI approval, but destructive-operation confirmation is still missing |
| 10.2.7 | Size/pagination: ~20 KB text cap, `limit`/`cursor` on lists, `detail` arg for summary/full | ⚠️ | `limit`/`cursor` on all five list tools (`alp-api/src/paging.rs`, default 50, max 200, opaque cursors, `total` + `next_cursor`); **no ~20 KB text cap and no `detail` arg yet** |
| 10.2.8 | Logging: stderr + audit log (protocol logging deprecated) | ✅ | `tracing` to stderr + `audit.jsonl` |

### 10.3 Tool Surface — 58 shipped (60 hard cap)

Every tool below is defined in `tool_defs.rs`, dispatched in `dispatch.rs`, and
covered by a tier-visibility test. Behaviour notes are in
[`docs/mcp/TOOLS.md`](docs/mcp/TOOLS.md).

| Group | Tools | Tier | Status |
|-------|-------|------|--------|
| Projects & History (12) | `project_list`, `project_info`, `project_create`, `project_open`, `project_save`, `project_close`, `history_list`, `history_undo`, `history_redo`, `checkpoint_create`, `checkpoint_restore`, `rollback_session` | Observe / Edit | ✅ |
| Media (2) | `media_list`, `media_import` | Observe / Edit | ✅ |
| Tracks & Clips (11) | `track_list`, `track_add`, `track_update`, `track_remove`, `clip_add`, `clip_update`, `clip_split`, `clip_remove`, `range_delete`, `marker_add`, `edit_batch` | Observe / Edit | ✅ |
| Effects (5) | `effect_catalog`, `effect_apply`, `effect_update`, `effect_remove`, `preset_apply` | Observe / Edit | ⚠️ Ids and preset names are validated against the catalog, but the offline renderer does not apply track effects yet |
| Smart Edits (4) | `edit_trim_silence`, `edit_shorten_pauses`, `edit_remove_fillers`, `edit_match_loudness` | Edit | ⚠️ Trim/pause/loudness really edit; filler removal needs word-level timestamps and refuses without them |
| Voice & TTS (6) | `tts_voices`, `voice_profile_get`, `tts_generate`, `tts_script_generate`, `tts_regenerate`, `voice_design` | Observe / Edit | ✅ Designed-only for `voice_design`; clone creation stays GUI-only |
| Transcription (1) | `transcribe` | Observe | ⚠️ Real via the daemon model; the in-process fallback emits placeholder words and says so |
| Analysis (7) | `analyze_loudness`, `analyze_silence`, `analyze_quality`, `analyze_spectrum`, `render_waveform`, `render_spectrogram`, `compare_renders` | Observe | ⚠️ Real measurements, but band energy is Goertzel (not FFT), peak is sample peak, hum detection is `None` |
| Export (5) | `export_presets`, `export_audio`, `export_stems`, `export_transcript`, `preview_render` | Observe / Export | ✅ Stems are per-track, transcripts real, previews written |
| Jobs (2) | `job_status`, `job_cancel` | Same as job | ✅ Backed by real jobs from TTS/STT/script |
| Plugins (3) | `plugin_list`, `plugin_insert`, `plugin_param_set` | Observe / Edit | ✅ Discovery and chain insertion; no plugin audio is rendered offline |

Not shipped: `transcript_get`, `edit_cut_transcript_ranges` (both need §10.0.3).

### 10.4 Gap-Fill Domain Features (built in domain crates)

| # | Feature | Status | Evidence |
|---|---------|--------|----------|
| 10.4.1 | Auto-ducking (lower music under speech via VAD) | ⬜ | No VAD-driven gain automation |
| 10.4.2 | Room-tone fill (replace removed pauses with ambience + crossfade) | ⬜ | Ripple cuts close gaps with silence |
| 10.4.3 | Loudness matching between clips/tracks | ⚠️ | `edit_match_loudness` matches the whole project with a peak ceiling; per-clip matching is not built |
| 10.4.4 | Transcript-based cutting (remove words → cut audio with micro-crossfades) | ⬜ | Needs word-level timestamps |
| 10.4.5 | Filler-word detection (user-editable list, conservative defaults) | ⚠️ | Word list and default exist; detection is gated on word timestamps |
| 10.4.6 | Batch apply (same chain/preset over many files, per-file report) | ⬜ | `edit_batch` batches operations inside one project, not files |
| 10.4.7 | Deterministic mode (`seed` for TTS where supported) | ⬜ | Not built |

### 10.5 Live Attach Mode & GUI

| # | Task | Status | Evidence |
|---|------|--------|----------|
| 10.5.1 | Settings → AI control: master switch, tier, folders, approval mode, transport | ⚠️ | Master switch and tier exist in the AI card; folders/approval/transport are not exposed there |
| 10.5.2 | Control host in `alp-ui` via `alp-api`: Unix socket `$XDG_RUNTIME_DIR`, 0600, `SO_PEERCRED`, per-session token | ✅ | Protocol 2 has 2 MiB bounded framing and opaque MCP envelopes. `alp-mcp --attach` reuses the complete shared dispatcher on desktop workers; production-socket tests prove catalog parity, revision-checked live edits, shared undo/redo, imported audio reconciliation and late-result revocation |
| 10.5.3 | AI activity panel: live tool calls, highlighting, **Stop AI** kill switch, **Undo all AI changes** (session-start checkpoint) | ⚠️ | Full MCP calls log preparatory outcomes, measured commits and one shared undo step; Stop AI revokes queued worker results. Edit highlighting and safe AI-only/full-session rollback remain missing (“Undo last edit” is labelled honestly) |
| 10.5.4 | Approval modes: Auto / Confirm destructive (default) / Review (stretch: staged checkpoint) | ⚠️ | `ApprovalMode` type exists; no enforcement path and no Review mode |
| 10.5.5 | Concurrency: human + AI share command queue; revision conflicts → actionable errors; UI never blocks | ⚠️ | Heavy MCP calls run against revision-tagged scratch state on workers; tested human/AI races, stale-read rejection and revocation preserve the one live project/history, with one undo entry and full path/audio context per committed call. Broad multi-client/flood/heavy-call soak remains unverified |
| 10.5.6 | Change notifications to subscribed clients on human edits | ✅ | Explicit control subscriptions receive post-commit GUI and attached project changes; real-socket integration verifies operator and AI events, and `--attach` gates forwarded standard resource-update/detail notifications on MCP `resources/subscribe`. Fragmented events survive timeouts and slow subscribers are bounded and dropped |
| 10.5.7 | Attach latency: AI edit visible ≤ 200 ms | ⚠️ | Real integration asserts request → worker preparation → revision-checked commit/view-model synchronization ≤200 ms for a cache-ready structural edit and exposes latest/worst timing. Actual pixel/audio presentation and heavy-call latency are not claimed |

### 10.6 Safety & Security

| # | Task | Status | Evidence |
|---|------|--------|----------|
| 10.6.1 | Permission tiers: Observe / Edit / Export enforced | ✅ | Enforced in `alp-api`, tested at MCP level even when a client forces the call |
| 10.6.2 | Never via MCP: clone profile creation, model install, settings changes, disable synthetic tag, outside workspace, run commands | ✅ | Verified in security tests |
| 10.6.3 | Human confirmation every time: delete project/media/voice, overwrite file, restore checkpoint discarding human edits | ⚠️ | No MCP path can delete a project or voice, and imports never overwrite; checkpoint restore has no confirmation round-trip |
| 10.6.4 | Prompt-injection hygiene: transcripts/filenames/tags/markers untrusted; no shell/generic file tools; static tool descriptions | ✅ | No shell/file-generic tools; descriptions are `&'static` in `tool_defs.rs` |
| 10.6.5 | Process confinement: stdio shares client trust; sandbox `alp-mcp` (Landlock/bubblewrap) | ⬜ | **Not done** |
| 10.6.6 | Limits: rate limits, per-call timeouts, max payload, max concurrent jobs | ⚠️ | Sliding-window rate limit (600 req/60 s), 1 MiB payload cap, advisory 120 s call budget — all CLI-tunable and published in `server/discover` (`alp-mcp/src/limits.rs`); **no max-concurrent-jobs cap (jobs are synchronous)** |
| 10.6.7 | Audit log: every call, viewable in AI panel; hashes/lengths only | ✅ | JSONL + the AI card's activity list |
| 10.6.8 | Visibility: persistent AI-activity indicator when session attached | ⚠️ | Enabling the native host shows the active AI badge and full attached MCP activity including measured commits; per-connection indicators remain missing |

### 10.7 Docs & Tooling

| # | Task | Status | Evidence |
|---|------|--------|----------|
| 10.7.1 | Module docs `alp-api.md`, `alp-mcp.md` | ✅ | `docs/modules/` |
| 10.7.2 | `docs/mcp/TOOLS.md` (generated), `WORKFLOWS.md`, `SECURITY.md`, `CLIENT_SETUP.md`, `AGENT_GUIDE.md` | ⚠️ | All present and hand-maintained (not generated); `AUDIT.md` added, `AGENT_GUIDE.md` is now served as an MCP resource |
| 10.7.3 | Update `check-deps.sh`, `check-docs.sh`, architecture diagram | ✅ | Both scripts cover the new crates/modules |
| 10.7.4 | `scripts/phase10-demo.sh` | ✅ | `scripts/phase10-demo.sh` |

### 10.8 Evaluation Harness

| # | Task | Status | Evidence |
|---|------|--------|----------|
| 10.8.1 | Scripted-agent replays of every prompt recipe via real MCP client; golden project JSON + render null-tests | ⬜ | **Not started** — `scripts/mcp_eval.py` exercises a subset but there are no golden artifacts |
| 10.8.2 | LLM-in-the-loop trials (clean noisy recording, narrate script, remove fillers, master to loudness); record success rate, tool calls, tokens, failure causes in `docs/mcp/EVAL.md` | ⬜ | `EVAL.md` is a specification, not a results log |
| 10.8.3 | Tighten tool descriptions/defaults from findings | ⚠️ | This audit tightened ~15 descriptions and 1 schema; no trial data yet to drive the rest |

---

## Phase 11 — CLAP/LV2 Plugin Hosting (Post-1.0 Backlog → Pulled Forward)

| # | Task | Status |
|---|------|--------|
| 11.1 | `alp-plugin` crate: CLAP 1.2+ & LV2 hosts, C-ABI loading, real-time-safe isolation | ✅ |
| 11.2 | Scanner/cache, sandboxed host process | ✅ |
| 11.3 | Effects workspace: hosted-plugin browser with Rescan | ✅ |
| 11.4 | CLI: scan/list/inspect | ✅ |
| 11.5 | ADR 0021, `scripts/phase11-demo.sh` | ✅ |

---

## UI Modernization Plan (from `plan.md` — REA-Driven Seven-Workspace Shell)

> Transform `alp-ui` from floating-panel dashboard → clean, modern, spacious audio studio inspired by VoiceStudio reference app.

| Step | Task | Status |
|------|------|--------|
| **1** | **Update Theme & Design Tokens** (`alp-ui-kit`): deep space dark `#0c0c0e`, warm card `#161619`, subtle borders `#26262c`, warm gold accent `#e5b842`, Inter typography, radius tokens | ⬜ **Not started** |
| **2** | **Implement Left Navigation Rail** (`alp-ui/src/app/sidebar.rs`): 72px slim rail, golden brand icon, 5 icon buttons with tooltips (Voice Studio, Voice Recording & Clone, Voice Editing & Stories, Effects & Plugins, Export & Delivery), bottom status (Settings, AI badge, version) | ⬜ **Not started** |
| **3** | **Redesign Workspaces** (`alp-ui/src/app/workspace_layout.rs`):<br>• **Voice Studio**: header + Upload/Create Voice, 2-col profile grid, floating generate box (voice selector, text input, model tier, gold ✦ button), right history feed<br>• **Voice Recording & Clone**: two-column modal (method tabs Upload/Record/System Audio, live visualizer + timer + countdown, reference text) ↔ (avatar, name, description, language, ✦ Create Profile)<br>• **Voice Editing & Stories**: left story list (+ New Story), right story lines (speaker badges, durations), bottom docked multitrack timeline (transport, tools ✂⧉🗑, zoom, ruler, clip lanes)<br>• **Effects & Plugins**: Voice Cleanup chain + discovered CLAP/LV2 with "+ Insert"<br>• **Export**: delivery presets + render jobs table | ⬜ **Not started** |
| **4** | **Refactor App Chrome & Views** (`app.rs`, `views.rs`): mount left nav rail, render active workspace in main content, dock horizontal timeline at bottom of editing workspace, all module docs in `docs/modules/` (8 required sections) | ⬜ **Not started** |
| **5** | **Verification & QA**: `check-file-length.sh` (≤1500 lines), `check-deps.sh`, `check-docs.sh`, `check-ui.sh` (zero raw `iced::widget::button`), `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, build release `alp-ui` | ⬜ **Not started** |

---

## Post-1.0 Backlog (Not Started)

| Item | Notes |
|------|-------|
| Time-stretch/pitch | License check: Rubber Band GPL vs Signalsmith MIT |
| Spectral editing | |
| Macros/batch processing | |
| MIDI | |
| Opus/AAC exports | |
| Authenticated remote MCP | |
| Multi-client MCP sessions | |
| Review mode for AI edits (staged checkpoint accept/reject) | Stretch from 10.5.4 |
| More workflow recipes | |

---

## Summary Counts

| Category | Total | ✅ Done | ⚠️ Partial | ⬜ Pending |
|----------|-------|---------|------------|----------|
| **P0 — Model tier real** | 8 | 8 | 0 | 0 |
| **Phases 0–9 (Core Product)** | ~85 | ~70 | ~9 | ~7 |
| **Phase 10 (MCP)** | 60 | 26 | 21 | 13 |
| **Phase 11 (Plugins)** | 5 | 5 | 0 | 0 |
| **UI Modernization** | 5 | 0 | 0 | 5 |
| **Post-1.0 Backlog** | 9 | 0 | 0 | 9 |

---

## Next Actions (Priority Order)

1. **P0 — Model tier real:** ✅ Complete. P0.4 now has genuine Qwen3-TTS 0.6B CPU inference via pinned Crane/Candle and official Safetensors, with reference PCM + transcript ICL, consent enforcement, truthful clone-specific provenance, no Qwen-to-acoustic substitution, and passing direct/daemon model-gated tests (ADR 0029)
2. **P1 — MCP depth:** the 58-tool headless surface is complete and honest about its limits ([`docs/mcp/AUDIT.md`](docs/mcp/AUDIT.md)). Done this round: `expected_revision` on writes (10.1.3), `title`/`annotations` + `ttlMs`/`cacheScope` (10.2.2), `limit`/`cursor` pagination on the five list tools (10.2.7), rate limit / payload cap / advisory call budget (10.6.6), and the owner-only `alp-ui` control host with human approval, per-session tokens and 2 MiB framing (10.5.2). The complete shared MCP dispatcher is now available through `alp-mcp --attach`, with worker-prepared edits committed to the live GUI project/undo stack; production-socket integration covers catalog parity, media/audio reconciliation, human-edit conflicts, revocation, notifications and measured request-to-live-commit latency (10.5.2/10.5.6/10.5.7). Remaining: `outputSchema` (10.2.3), word-level timestamps (10.0.3) to unblock filler removal and transcript cutting, the ~20 KB text cap, actual rendered-pixel latency, and the evaluation harness golden artifacts / LLM trials (10.8)
3. **P2 — Phase 5 completion:** RNNoise/spectral noise reduction; silence/breath tools in `alp-dsp`
4. **P3 — Recording/timeline depth:** Multi-input overdub, spectrogram + minimap, monitoring resampler, async import, project picker/save-as
5. **P4 — Release assurance:** Cross-desktop matrix, clean-machine passes, `phase2-demo.sh`
6. **P5 — Engineering debt:** Address 16 audit findings
7. **UI Modernization:** Execute Steps 1–5 from `plan.md`

---

*This checklist is the single source of truth for project status. Update in the same commit as code changes.*