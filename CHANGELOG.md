# Changelog

All notable changes to Audio Labs Pro. Format loosely follows Keep a Changelog;
versions follow the release milestones in the build plan.

## [Unreleased]

- Feature: Phase 13 implemented high-resolution Short-Time Fourier Transform (STFT) spectrogram engine in `alp-dsp::spectral` with configurable FFT sizes (512, 1024, 2048, 4096, 8192), hop sizes, analysis window functions (Hanning, Hamming, Blackman-Harris, Rectangular), dBFS PSD normalization, and Linear/Logarithmic/Mel frequency bin aggregation
- Feature: Phase 13 integrated timeline multi-view track lanes in `alp-session` and `MultitrackCanvas.tsx`, supporting `Waveform`, `Spectrogram`, and `Split` (dual 50/50 waveform + spectrogram multi-view) with track header view mode selector buttons
- Feature: Phase 13 built 2D time-frequency spectral selection box tool (`SpectralSelectionDto`) calculating geometric center frequency \( f_c = \sqrt{f_1 \cdot f_2} \), bandwidth \( \Delta f = f_2 - f_1 \), and filter quality factor \( Q = f_c / \Delta f \) with live floating HUD readout
- Feature: Phase 13 added clip stereo pan automation envelope editing with keyframe nodes and non-destructive slip editing (`slip_offset_secs`, Alt+drag)
- Feature: Phase 13 exposed 5 typed Tauri IPC commands in `alp-desktop` (`get_clip_spectrogram`, `timeline_set_track_view_mode`, `timeline_set_spectral_selection`, `timeline_set_clip_pan_envelope`, `timeline_slip_clip`)
- Feature: Phase 13 implemented 5 autonomous MCP agent tools (`timeline_spectral_analyze`, `timeline_spectral_select`, `timeline_set_view_mode`, `clip_set_pan_envelope`, `clip_slip`), expanding the autonomous MCP tool suite to 68 tools
- UX & Frontend: Phase 13 built `SpectrogramCanvas.tsx` with calibrated Magma, Inferno, and Viridis heatmaps, logarithmic frequency grid overlays (50 Hz to 20 kHz), and hover inspector
- Generative UI: Phase 13 generated interactive live multi-view playground artifact `phase13_spectral_multiview.html` showcasing colormap switching, 2D spectral box selection, pan automation, and STFT specs
- Verification: Phase 13 verified pure-tone spectral peak detection, window symmetry, monotonic binning, session operations undo/redo, 8/8 timeline tests, and workspace compilation with zero `unsafe` hand-written code

- Packaging: Phase M8 updated Linux packaging manifests for `.deb` (`libwebkit2gtk-4.1-0`, `libgtk-3-0`), `.rpm` (`webkit2gtk4.1`, `gtk3`), and Flatpak (`org.gnome.Platform // 48` runtime, `org.gnome.Sdk`, `org.freedesktop.Sdk.Extension.node20`, target binary `alp-desktop` installed as `audio-labs-pro`)
- Metadata: Phase M8 recorded `0.0.1-alpha.8` release notes in AppStream metadata (`io.github.zvirox.AudioLabsPro.metainfo.xml`)
- Cutover & Cleanup: Phase M8 cleanly removed legacy iced crates (`crates/alp-ui`, `crates/alp-ui-kit`) and workspace dependencies (`iced`, `iced-cube`, `lilt`), achieving complete cutover to the Tauri 2 + React desktop shell
- Guard Tooling: Phase M8 updated `scripts/check-deps.sh` for 12 ALP crates and updated `scripts/check-ui.sh` to verify frontend kit compliance and zero legacy iced references
- Documentation Debt: Phase M8 resolved documentation debt across `docs/TROUBLESHOOTING.md` (WebKitGTK distro packages, NVIDIA/Wayland webview flags), `docs/USER_GUIDE.md` (build target updated to `alp-desktop`, React studio workflows, canvas timeline), `docs/KEYBOARD_SHORTCUTS.md` (full shortcut parity verified), `THIRD_PARTY.md` (Tauri 2 crates, WebKitGTK 4.1, and npm package inventory; iced marked retired), and `docs/architecture/ARCHITECTURE.md`

- Feature: Phase M7 implemented headless `AiControlManager` in `alp-session::ai` hosting owner-only Unix domain control socket at `$XDG_RUNTIME_DIR/alp/control.sock` (mode 0600) with per-session token authentication (`ALP_CONTROL_TOKEN`)
- Feature: Phase M7 enabled full Model Context Protocol (MCP) tool dispatching on dedicated worker threads (`alp-attached-mcp-{client_id}`) executing against base-revision scratch snapshots
- Reliability: Phase M7 enforced live atomic transaction commit into `EditSession`, where multi-step agent operations commit as a single undo entry, with automatic rejection of stale writes on revision conflict
- Feature: Phase M7 exposed 6 typed AI IPC commands in `alp-desktop` (`get_ai_status`, `toggle_ai_control`, `set_ai_permission_tier`, `respond_ai_approval`, `get_ai_activity_log`, `undo_ai_change`) and continuous background event pump emitting `session_revision_changed`, `ai_client_approval`, and `ai_activity_event`
- UX & Frontend: Phase M7 upgraded `SettingsWorkspace` with active/standby MCP host indicator, 1-click token copy, shell attach command snippet generator, permission tier selector (`Observe`, `Edit`, `Export`), latency metrics, interactive client approval prompt banner, and live AI audit log stream
- Verification: Phase M7 verified full MCP catalog parity (`tools/list`), live undoable edits (`tools/call track_add`), and MCP rollback (`tools/call history_undo`) with `test_attached_mcp_full_catalog_and_tool_commit` passing in `alp-session`

- Feature: Phase M6 implemented high-performance HTML5 Canvas 2D multitrack timeline editor (`MultitrackCanvas.tsx` and `MultitrackWorkspace.tsx`) with track lanes, non-destructive clips, waveform overviews, fade curves, volume automation envelopes, timecode ruler, loop boundaries, cue markers, and playhead beam
- Feature: Phase M6 developed framework-free mathematical hit-testing and drag interaction state machine (`frontend/src/timeline/timelineMath.ts`, `dragLogic.ts`) with magnetic snapping to markers, playhead, and adjacent clip boundaries
- Verification: Phase M6 created direct Node test suite (`frontend/src/timeline/timelineMath.test.ts`, `npm test`) validating coordinate projections, 8px handle detection, fade geometry, and IPC command payloads (8/8 tests passing)
- Feature: Phase M6 wired 16 typed timeline IPC commands into `alp-desktop` and `alp-session` (`get_timeline_state`, `timeline_undo`, `timeline_redo`, `timeline_split_clip`, `timeline_delete_selected`, `timeline_cut`, `timeline_copy`, `timeline_paste`, `timeline_move_clip`, `timeline_trim_clip`, `timeline_set_clip_fades`, `timeline_set_clip_envelope`, `timeline_set_loop_range`, `timeline_add_marker`, `timeline_remove_marker`, `timeline_select_clip`, `timeline_set_selection`)
- Verification: Phase M6 verified keyboard editing shortcuts parity (`Ctrl+Z`, `Ctrl+Y`, `Ctrl+B`, `Delete`, `Ctrl+C`, `Ctrl+X`, `Ctrl+V`, `R`, `Ctrl+A`, `Space`, `Home`), identical clip ID restoration across redo (`timeline_operations_undo_redo_and_clip_id_preservation`), and zero hand-written `unsafe` code in `alp-desktop`
- Feature: Phase M5 ported all 7 studio workspaces to React/TypeScript/Tailwind using kit primitives: Settings (`SettingsWorkspace`), Models (`ModelsWorkspace`), Voices (`VoicesWorkspace`), Generate (`GenerateWorkspace`), Effects (`EffectsWorkspace`), Captures (`CapturesWorkspace`), and Stories (`StoriesWorkspace`)
- Security & Safety: Phase M5 strictly preserved the affirmative human-only legal consent gate for zero-shot voice cloning (`CloningConsentModal`), requiring non-empty speaker names, attester identity, reference audio, and explicit policy agreement
- Feature: Phase M5 added hosted CLAP / LV2 plugin discovery and rescan UI, 6-stage vocal cleanup chain with stage bypass toggles, and signal path metering in Effects workspace
- Feature: Phase M5 integrated live VAD, input meter health audit, takes list, and interactive word-level transcript editor with click-to-seek and segment cuts in Captures workspace
- Feature: Phase M5 built multi-character chapter dialogue script editor, cast assignment, line status tracking, and batch synthesis in Stories workspace
- UX: Phase M5 unified workspace switching and floating panel overlay in `SubtabTopBar` with one-click view toggling and full keyboard accessibility
- Feature: Phase M4 implemented `alp_session::TransportController` and wired live audio playback to `alp-desktop` IPC (`play_transport`, `pause_transport`, `stop_transport`, `seek_transport`, `set_transport_volume`, `set_transport_loop`, `get_transport_state`)
- Feature: Phase M4 established real-time calibrated stereo meter streaming over Tauri 2 `Channel` at ~30 Hz (`subscribe_meters`), paired with client-side playhead interpolation at 60–120 fps (`useTransport`) via monotonic timestamp anchors
- Feature: Phase M4 built high-performance `WaveformCanvas` in HTML5 Canvas 2D with dynamic decimation, level-of-detail pyramid selection (256, 1024, 4096 spl), interactive scrubber seeking, and zero-idle-draw optimization
- Feature: Phase M4 built `TimelineWaveformPanel` integrated into floating panels with real-time 1-hour benchmark testing mode (675,000 min/max buckets @ 48 kHz)
- Verification: Phase M4 verified 1-hour waveform autoscroll gate (60.0 FPS sustained) and 10-minute playback soak with 0 dropped callbacks (`test_10_minute_playback_soak_no_dropped_callbacks`)
- Feature: Phase M3 built studio component kit (`frontend/src/kit/`: Button, Toggle, Slider, Input, Tabs, Dialog, Tooltip, Menu, Meter, Card) and dual-axis layout shell (56px PrimaryRail, 40px SubtabTopBar, 280px InspectorDrawer, 64px DockedPlayer)
- Feature: Phase M3 implemented floating panel container system (`PanelContainer`) supporting drag, resize, z-order focus, collapse, and docking, synchronized with `workspace.json` through `save_workspace_state` and `reset_workspace_layout`
- Feature: Phase M3 integrated `cmdk` Command Palette (`Ctrl+K`), Shortcuts modal (`F1`), Theme cycling (`Ctrl+D`), and Reduce motion (`Ctrl+Shift+M`) with complete keyboard parity
- Feature: Phase M2 created `alp-desktop` native desktop shell (Tauri 2) with safe KWin blur via `x11rb`, translucent tint fallback (alpha >= 0.70), and zero `unsafe` hand-written code under `#![deny(unsafe_code)]`
- Feature: Phase M2 scaffolded `frontend/` (Vite, React 19, TypeScript strict mode, Tailwind CSS v4, Lucide icons) with full studio design token palette and live IPC bridge to `alp-session`
- Packaging: Phase M2 verified early Flatpak de-risk manifest (`packaging/flatpak/throwaway-tauri-derisk.yaml`) on GNOME Platform 48 with WebKitGTK 4.1 and test tone playback
- Refactor: Phase M1 extracted headless `alp-session` crate (`EditSession`, `JobQueue`, `PersistedWorkspace`, `PanelManager`) with zero GUI dependencies, split `app.rs` from 1,054 down to 669 lines, and verified all 144+ workspace tests and MCP live attach parity
- Spike: Phase M0 Tauri 2 + WebKitGTK feasibility spike and live benchmarks (GO gate passed; see `docs/spikes/PHASE-M0-RESULTS.md`)
- Docs: UI migration plan and ADR 0013 accepted

### Planned (Phase 13–19 Adoption Roadmap)

- **Phase 13 (`v0.0.1-alpha.8`):** Interactive Multitrack timeline canvas, clip splitting/slip/ripple, simultaneous Waveform + STFT Spectrogram, and 2D time-frequency spectral selection tool.
- **Phase 14 (`v0.0.1-alpha.9`):** Subtractive spectral denoiser with noise profiling, 2D spectral notch/repair, realtime insert FX racks, and high-fidelity WSOLA time-stretching.
- **Phase 15 (`v0.0.1-alpha.10`):** Zero-shot voice cloning embeddings, prompt-driven voice design, persona cards & avatars, and custom pronunciation lexicon with IPA/SSML.
- **Phase 16 (`v0.1.0-beta.1`):** Audiobook Studio with EPUB/Markdown ingestion, multi-character cast tagging, cadence pacing rules, and M4B container packaging with QuickTime chapter markers.
- **Phase 17 (`v0.1.0-beta.2`):** Video Dubbing with Demucs v4 4-stem separation, sync-locked label tracks (SRT/VTT), speech-to-speech dubbing with rate fitting, and FFmpeg video remux.
- **Phase 18 (`v0.1.0-beta.3`):** VST3 plugin hosting engine, native X11/Wayland plugin window embedding, and scriptable DSP macro batch processing.
- **Phase 19 (`v1.0.0-rc.1` / `v1.0.0`):** Full MCP autonomous agent tool coverage across all studio features, multi-step eval benchmarks, 24-hour stability soaks, and multi-distro release packaging.
- Complete roadmap and execution checklists: see [`docs/ADOPTION_PLAN.md`](docs/ADOPTION_PLAN.md) and [`REAVSA.md`](REAVSA.md).

## [0.0.1-alpha.7] - 2026-11-23

### Added

- **Dual-Axis Navigation System:** 56px Vertical Primary Rail for top-level studio switching (`Voice Lab`, `Multitrack Studio`, `Audiobook Studio`, `Dubbing Studio`, `Audio Tools`, `Effects & Plugins`, `Settings`) paired with a 40px Horizontal Subtab Top Bar for contextual sub-navigation, liberating full horizontal width for multitrack timelines, waveforms, and scripts.
- **Collapsible Right Inspector Panel:** 280px drawer toggled via `Ctrl+B` or header button for fine-grained property inspection without permanent width penalty.
- **`EnvelopeCurve` and `InterpolationMode` (`alp-core::envelope`):** Linear, Smooth cosine S-curve, and Exponential decibel curves with multi-point volume/pan evaluation.
- **`Clip::evaluate_gain_at` (`alp-core::project`):** Real-time compound gain calculation factoring clip gain, fade-in, fade-out, and envelope curves.
- **`TrackKind` (`alp-core::project`):** Specialization across `AudioTrack`, `LabelTrack` (synchronization and cue annotations), and `TimeTrack` (tempo and dynamic speed curves).
- **`SampleBlock` and `BlockStoreSchema` (`alp-core::block`):** 64KB block-based audio chunking and SQLite WAL schema for instantaneous zero-copy clip splitting and crash-safe undo transactions.
- **Comparative Architecture & Adoption Documentation:** Added `REAVSA.md` and `docs/ADOPTION_PLAN.md` with complete Phase 12–19 master release timeline.

## [0.0.1-alpha.6] - 2026-11-23

### Added

- **Voice-profile parity (alpha.6, phase 1).** New MCP tools:
  `voice_profile_update`, `voice_profile_lock`, `voice_profile_unlock`.
  `VoiceProfileDetail` now carries the new `locked` flag.
- **`tts_batch`**: synthesize a list of `{voice_id, text, speed?}` requests — one
  `JobManager` job per request, run sequentially, cancellable via `job_cancel`.
- **`audiobook_encode`**: synthesize a script of `{text, voice_id, speed?, gap_secs?}`
  segments with their voices, place the clips on a dedicated `Audiobook` track,
  and export the master with the audiobook delivery preset.

### Changed

- `VoiceProfile` (alp-voice) gains a `locked: bool` field (serde-defaulted so
  stored profiles load without it); set to `false` in all constructors.

## [0.0.1-alpha.5] - 2026-11-22

### Changed

- **Minimal, responsive studio UI.** The dark palette now uses calm slate
  surfaces and the light palette paper surfaces, with restrained workspace
  accents, larger type, a labelled navigation rail, higher-contrast secondary
  text and a compact waveform ruler that no longer paints over peak data.
  Tooltips are opaque and viewport-clamped; grids, padding and dock controls
  step down on compact windows, and the initial window fits the primary monitor.
- **Visible workspace motion.** The existing lilt toggles now drive an 8 px
  workspace settle transition through a layout-safe translation wrapper, with
  reduced motion collapsing it to an instant state.

### Added

- WCAG AA contrast tests for every text role and accent ink in both themes.
- README status, configuration, FAQ and troubleshooting sections aligned with
  the shipped 58-tool MCP surface and current release evidence.

## [0.0.1-alpha.4] - 2026-11-21

### Added

- **`alp-core::CompositeCommand` and stack grouping.** A batch edit is now a
  single undo entry: `CommandStack::take_last(count)` removes the commands a
  multi-step operation just applied, and `push_applied` pushes the
  `CompositeCommand` that wraps them without re-applying them. Reverting the
  group walks the commands in reverse; a mid-group apply failure reverts what
  the group already did. `ApiSession::group_commands_since` builds the group
  from the stack depth recorded before the operation.
- **`edit_batch` is a real, atomic batch.** It accepts an `operations` array of
  eight typed operations (`track_update`, `clip_update`, `clip_remove`,
  `clip_split`, `marker_add`, `range_delete`, `effect_apply`, `effect_remove`),
  routes each to the real editing operation, and reports the true
  `operations_applied`. A failing operation rolls the whole batch back and the
  error states that no changes were applied.
- **Per-track stem exports.** `export_stems` renders each track in isolation and
  writes `stem_NN_<track>.wav`; each report carries that stem's own peak, so a
  quiet track is provably quiet.
- **Real transcript and preview exports.** `export_transcript` renders the
  project, transcribes it, and writes actual `text`/`srt`/`vtt` content (the
  report echoes the text). `preview_render` writes a real WAV slice into
  `media/` and reports that slice's own peak.
- **ASCII picture renders.** `render_waveform` and `render_spectrogram` return
  the art they computed in `ascii` (plus `band_centers_hz` and `range_db`) in
  place of the old stub `data_uri`.
- **`tts_script_generate` and `tts_regenerate` produce audio.** A JSON script
  (object with `segments` or a bare array) becomes clips on a `Script` track in
  one undo step; regeneration replaces a clip's audio in place at its original
  start.
- **Real effect-catalog validation.** `effect_apply` and `preset_apply` reject
  ids/names the catalog does not offer and list the known ones.
- **`docs/mcp/AUDIT.md`** records what was fabricated, what was fixed, and what
  stays honestly partial; `docs/mcp/TOOLS.md` documents all 58 tools.
- **Optimistic concurrency on every write.** All 58 tool schemas gain an
  `expected_revision` argument on mutating tools: a write that names a stale
  revision fails with a concurrency conflict naming `project_info` as the
  recovery, before anything changes. Reads ignore the argument — the read is
  what an agent needs to recover, so failing it would be self-defeating.
- **Cursor pagination on the five list tools.** `project_list`, `track_list`,
  `media_list`, `history_list` and `plugin_list` accept `limit` (default 50,
  max 200) and `cursor`, and return `{ items, total, offset, next_cursor? }`.
  Cursors are opaque `v1<hex>` tokens; a malformed cursor is a validation error
  rather than a silent restart, and an offset past the end is an empty page
  with the real total (`alp-api/src/paging.rs`, five unit tests).
- **Tool titles, annotations and cache hints.** Every tool carries a `title`
  plus `annotations` (`readOnlyHint`, `destructiveHint`, `idempotentHint`,
  `openWorldHint`) from one metadata table in `tool_defs.rs`, parity-tested
  against the advertised surface; `tools/list` returns `ttlMs`/`cacheScope`.
- **Boundary limits at the MCP transport.** A sliding-window rate limit
  (default 600 requests / 60 s), a 1 MiB payload cap and an advisory 120 s
  call budget, all CLI-tunable (`--requests-per-window`,
  `--max-payload-bytes`, `--call-timeout-ms`) and published in
  `server/discover` with the caller's remaining budget. An over-budget call
  still returns its real result, with a note reporting the measured overrun
  (`alp-mcp/src/limits.rs`, five unit tests).
- **Owner-only live GUI control host and full MCP attach.** `alp-api::control`
  binds `$XDG_RUNTIME_DIR/alp/control.sock` through a `0700` parent, sets `0600`,
  checks `SO_PEERCRED`, requires a fresh random token and human client approval,
  uses protocol 2 with 2 MiB bounded frames, and revokes active clients on Stop
  AI. `alp-mcp --attach` forwards complete JSON-RPC envelopes over that socket
  without opening a disk project; the desktop runs the same 58-tool dispatcher
  as headless mode on per-client workers against revision-tagged scratch
  snapshots. Only an unchanged live revision can commit to the one
  `EditSession`, producing one ordinary undo entry; history tools use that live
  stack. Explicit resource subscriptions publish committed operator and AI
  changes with measured request-to-commit timing. The four native project calls
  remain for compatibility.

### Fixed

- **`history_list` no longer invents history.** It reported "Undoable edit
  step #1"-style placeholders derived from an availability flag. It now walks
  the real undo/redo stacks via `CommandStack::undo_labels`/`redo_labels` and
  reports each command's actual label (`Rename track`, `Add clip`, …);
  `project_info.undo_steps`/`redo_steps` are real counts that agree with the
  list (MCP test `history_list_reports_the_real_command_labels`).

- **Attached undo restores the whole project context.** An attached edit's undo
  entry now carries the project root and the decoded source-audio cache, not
  just the project JSON. Undoing or redoing a copied project that reuses source
  UUIDs can no longer leave playback and autosave pointed at another project's
  media.
- **Stale attached reads no longer masquerade as current.** Every attached
  result reports `snapshotRevision` and `liveRevision`. A read that finishes
  after a newer commit now fails with both revisions instead of returning old
  data after the change notification.
- **Attached change events survive fragmentation and slow subscribers.** The
  proxy retains a partial frame across socket timeouts, while the host keeps at
  most 64 pending changes per subscriber and disconnects a client that falls
  behind. Bounded write timeouts keep Stop AI from waiting on a stalled reader.

- **Several MCP tools reported work they never did.** `export_stems` wrote the
  master mix N times, `export_transcript` wrote a one-line header,
  `preview_render` produced no file, the picture tools returned a stub,
  `edit_batch` reported `operations_applied: 1` while applying nothing, and
  `tts_script_generate` was an `Ok(())` no-op. Each now performs the operation
  it names, and tests assert the artefact exists.
- **Smart edits now edit.** `edit_trim_silence` and `edit_shorten_pauses` cut
  real ripple ranges and report measured before/after lengths;
  `edit_match_loudness` applies gain in the correct direction (the sign was
  inverted), re-measures, and respects a −1 dBFS sample-peak ceiling.
  `edit_remove_fillers` refuses when only placeholder words exist instead of
  cutting at invented positions.
- **Analysis stopped overstating its measurements.** The peak is labelled a
  sample peak (`true_peak_dbfs` → `peak_dbfs`), the noise floor is the
  10th-percentile 20 ms frame level, and `silence_ratio` became `active_ratio`
  (the fraction of frames actually above the threshold); `speech_ratio` was
  removed. `analyze_silence` honours the ≥100 ms minimum run length.
- **TTS/STT/script are job-backed.** `tts_generate`, `transcribe` and
  `tts_script_generate` submit and complete real `JobManager` jobs and return a
  `job_id`, so `job_status`/`job_cancel` operate on real work.
- **The `alp://effects/catalog` resource serves the live catalog**, and
  `alp://docs/agent-guide` serves `docs/mcp/AGENT_GUIDE.md` via `include_str!`;
  the `produce_podcast_episode` prompt is implemented.
- **AI controls now describe their actual transport and undo scope.** The card
  distinguishes the live native attach surface from the separate headless MCP
  process, Stop AI really revokes attached clients, and the one-step undo action
  remains labelled “Undo last edit” rather than claiming checkpoint rollback.

### Changed

- **Branding & Visual Identity:** Replaced legacy cassette logo with the new
  official Audio Labs Pro microphone & waveform identity across the desktop
  application shell, window icon, desktop integration assets, hicolor icon theme
  (scalable SVG and 48px to 256px HiDPI PNGs), packaging metadata, and documentation.
- **`MASTER_CHECKLIST.md` Phase 10 now matches the code.** It records the real
  58-tool headless surface, the honest partials, and the limited native control
  host boundary instead of the stale 22-shipped/no-host table.
- **The tool dispatch moved out of `server.rs` into `dispatch.rs`.** The server
  keeps transport, limits, the project lock and the concurrency guard; the
  module that maps a tool name to an operation is solely about that mapping.
  `server.rs` drops from ~1550 lines (over the 1500 gate) to ~400.

- **A working acoustic clone tier (P0, ADR 0028).** `CloneBackend` now has a
  real implementation. `alp-voice::voiceprint` measures a reference recording
  with actual signal analysis — normalized-autocorrelation pitch tracking
  (with octave correction and parabolic interpolation), a hand-written radix-2
  FFT for the long-term average spectrum, and 16 log-spaced band gains
  between 100 Hz and 8 kHz — and encodes the result into the existing
  256-dimensional `VoiceProfile::cached_embedding` slot, so a cloned profile
  stays a plain portable profile JSON. `alp-voice::acoustic`
  (`AcousticCloneBackend`) renders speech as a 48 kHz harmonic stack whose
  per-harmonic weights are the product of a three-formant resonance envelope,
  the measured band gains, and a glottal `1/k` rolloff: the output genuinely
  tracks the reference's timbre rather than imitating it by ear. This tier
  needs no model download, so cloned voices work on a clean machine.
- **Profile-conditioned synthesis over the daemon protocol.** A cloned
  profile's speaker conditioning cannot be expressed by a voice id, so
  `InferRequest::SynthesizeWithProfile` carries the whole profile and
  `InferClient::synthesize_with_profile` sends it. `alp-infer` gained a third
  engine slot (`CloneEngine`) beside synthesis and transcription; loading
  `qwen3-tts` populates it without disturbing the other two. The daemon
  enforces the human-only consent gate when it receives a cloned profile, and
  the in-process fallbacks in `alp-api` and `alp-ui` enforce the same rule so
  the guarantee does not depend on the daemon being reachable.
- **The clone tier is reachable headlessly.** `alp-cli tts --profile <id>`
  renders any stored profile, including a cloned one, with the same
  daemon-first routing and acoustic fallback. *Creating* a cloned profile
  stays GUI-only, as the build plan requires: the consent attestation must be
  human-only, and the CLI must not satisfy it.
- **One source for the synthetic-voice provenance tag.** Added
  `AudioMetadata::synthetic_speech(engine)`, which builds the watermark, the
  comment and the synthetic flag together. Every TTS export path now uses it
  — the API, the CLI (`tts` and `script render`) and the UI — so the tag
  cannot be dropped by a path that forgot to set it. The MCP `export_audio`
  tool still exposes no metadata flags, so the tag remains impossible to
  disable through MCP.
- **Real Qwen3-TTS 0.6B neural cloning.** The pinned `crane-core` 26.9.0
  runtime executes the seven official Qwen Safetensors/config/tokenizer assets
  with CPU/F32 ICL from the actual reference recording and transcript. It runs
  in the worker's independent clone slot, returns mono 24 kHz PCM, and reports
  `active_clone_model = qwen3-tts`. API/CLI provenance names the clone engine,
  and Qwen requests fail explicitly rather than silently falling back to the
  still-available `acoustic-clone` tier. The workspace MSRV is now Rust 1.95,
  required by Crane's `sysinfo 0.39` transitive dependency. A real daemon test
  verified consent refusal, consented neural synthesis, output, status, and
  unchanged plain-TTS routing (ADR 0029).
- **Nested model installation paths.** `ModelManager::install_file` creates
  manifest subdirectories such as `speech_tokenizer/` before the atomic `.part`
  rename.
- **A dependency-free GGUF header reader.** `alp-infer::gguf` parses a GGUF
  v3 header (magic, version, counts and metadata) without any new crate. It is
  retained as a test-only parsing utility after Qwen moved to official
  Safetensors.

### Fixed (voice and Qwen)

- **The Qwen3-TTS model manifest pointed at files that do not exist.** The
  catalogue entry listed `gguf/qwen3-tts-0.6b-q4_k_m.gguf` and
  `onnx/speech_decoder.onnx` under `Qwen/Qwen3-TTS-12Hz-0.6B-Base` — that
  repository ships neither GGUF nor ONNX — and carried a placeholder SHA-256
  (`a1b2c3d4…`). The manifest now lists only the seven official Apache-2.0
  `Qwen/Qwen3-TTS-12Hz-0.6B-Base` assets, with verified sizes and SHA-256
  digests; the intermediate community-GGUF manifest has been superseded.
- **The heavy clone engine reported synthesis it could not perform.** The
  previous `qwen3` engine described a "reference-conditioned acoustic mode"
  that did not exist and presented formant synthesis as neural output.
  `Qwen3Engine` now executes real neural inference. An unavailable model or
  failed generation returns an error and cannot be labelled as Qwen audio.
- **The GUI's clone dialog saved a profile with no voiceprint.** It now
  measures the reference through `AcousticCloneBackend::create_profile`, so
  the saved profile carries real conditioning. The GUI's generation and story
  renders route cloned profiles through the profile-conditioned request.
- **The CLI's TTS exports carried no synthetic-voice tag.** `alp-cli tts` and
  `alp-cli script render` wrote untagged WAVs.

### Known gaps

- **Neural clone quality and performance remain workload-dependent.** Real
  Qwen3-TTS execution and routing are tested, but the socket test is not a
  perceptual speaker-similarity benchmark; debug CPU inference is slow.
- **The acoustic clone tier reproduces timbre, not identity.** It does not
  model accent, phrasing or prosody, and the result is clearly synthetic next
  to a neural voice. Every surface names it `acoustic-clone` so the
  distinction is never blurred.

## [0.0.1-alpha.3] - 2026-11-08

### Added

- **Neural noise suppression & spectral denoiser (Phase 5).** Implemented RNNoise
  neural suppression (`nnnoiseless` 0.5.2) with FIFO buffering for arbitrary block
  sizes and classical Spectral Subtraction (`SpectralDenoiser`) with noise profile
  capture, over-subtraction factor $\alpha$, and spectral floor $\beta$ in `alp-dsp::denoise`.
  Integrated `NoiseReduction` into the `VoiceCleanup` preset chain.
- **Word-level timestamp alignment for STT (Phase 7).** Added `WordTimestamp`
  to `alp-voice::TranscriptSegment`, decoded BPE tokens with speech-aligned frame
  boundaries in `alp-infer::moonshine`, and wired word ranges into automatic filler
  removal (`edit_remove_fillers`) in `alp-api::smart_edit_ops`.
- **Heavy voice cloning runtime: Qwen3-TTS (Phase 8).** Added `qwen3-tts` manifest
  to `standard_catalogue()` in `alp-voice` (`ModelKind::VoiceClone`, Apache-2.0,
  435 MB GGUF weights + 52 MB ONNX decoder). Implemented `Qwen3Engine` in
  `alp-infer::qwen3` with 256-lane L2-normalized acoustic speaker embedding extraction
  and conditioned neural speech synthesis (`CloneBackend` & `TtsBackend`), wired into
  daemon model loading.
  > **Corrected in Unreleased.** The manifest's file URLs and digests did not
  > correspond to any real file, and the engine did not execute the model — the
  > "conditioned neural speech synthesis" it described was formant synthesis
  > presented as neural output. The manifest now lists verified Apache-2.0
  > files, and the engine validates the installation and reports the missing
  > runtime honestly. See ADR 0028.
- **Synthetic audio watermark metadata header (Ethics transparency).** Added
  `synthetic: bool` and `watermark: Option<String>` to `alp-media::AudioMetadata`.
  WAV exports automatically embed RIFF INFO chunks (`ISFT`, `ICMT`) and FLAC exports
  embed Vorbis comments (`SYNTHETIC_VOICE=true`, `PROVENANCE=...`). Updated `docs/ETHICS.md`
  section 1.3 to declare that the provenance watermark is active and enforced across exports.
- **MCP server modularization & evaluation benchmark (Phase 10).** Split
  `alp-mcp::server` to extract `tool_defs.rs` under the 1,500-line limit. Authored
  the complete MCP evaluation benchmark in `docs/mcp/EVAL.md` and implemented the
  automated evaluation harness (`scripts/mcp-eval.sh` / `scripts/mcp_eval.py`)
  verifying protocol handshake, resources, prompts, tool schemas, permission tier
  gating, sandboxing jail, ethics clone exclusion, workflow execution, and session
  rollback with a 100% pass rate.
- **Architecture guards and module documentation.** Authored missing module specifications
  for all `alp-api` operations (`analysis_ops`, `effect_ops`, `export_ops`, `job_ops`,
  `project_ops`, `script_voice_ops`, `smart_edit_ops`, `track_clip_ops`), `alp-dsp::denoise`,
  `alp-infer::qwen3`, and `alp-mcp::tool_defs` with all 8 required sections, satisfying
  `check-docs.sh` (157 documents valid) and all workspace architecture guards.
- **Packaging and release build hardening.** Updated Flatpak runtime to Freedesktop 25.08
  with Rust 1.98.1 to satisfy modern inference dependency requirements (`ort 2.0.0-rc.15`
  MSRV 1.92), stripped comments from Debian binary control metadata for strict `dpkg-deb`
  conformance, and aligned workspace `rust-version = "1.92"`.
- **MCP media-to-timeline toolset (ADR 0027).** `media_list` (Observe),
  `media_import` (Edit), and `clip_add` (Edit) complete the
  generate-to-timeline chain over MCP: synthesized speech can now be
  imported onto the timeline headlessly. Imports decode to prove they are
  audio, re-encode Float32 WAV under a fresh source id (an import can never
  overwrite existing media), measure the peak, and register through the
  existing undoable `AddSource` command - so imports participate in undo,
  redo, and the audit log exactly like editor actions. `clip_add` appends a
  clip with source-rate-aware range math (offsets and durations interpret at
  the source's own sample rate; the timeline start uses the project rate),
  clamps gain to 0.0-4.0, allows legal overlaps, and reports the written
  range so callers can detect them. Zero `alp-core` changes: the tools reuse
  the already-validated `AddSource`/`AddClip` commands, and storage
  conventions match the editor's own importer byte-for-byte.
- **MCP voice toolset (ADR 0026).** `alp-api` gained a voice module backing
  five new `alp-mcp` tools: `tts_voices` and `voice_profile_get` (Observe —
  profile list with daemon state; consent reported only as a boolean and
  `cached_embedding` never exposed), `transcribe` (Observe — sandboxed
  daemon-first transcription with optional `text`/`srt`/`vtt` output and the
  built-in segmenter's placeholder-words admission kept honest), and
  `tts_generate` plus `voice_design` (Edit — WAV synthesis into the project
  `media/` directory with the engine that actually ran named in the result;
  voice design persists Designed profiles only, so cloned voice creation and
  import stay unreachable through MCP at any tier). `alp://project/active`
  is now actually served (it was advertised but unread), and a new
  `alp://voices` resource serves the same voice summary. Engine results run
  daemon-first with built-in fallback, mirroring the Phase 7 UI pattern;
  `tts_script_generate`, `tts_regenerate`, `transcript_get`,
  `export_transcript`, and the job tools are deferred with reasons rather
  than stubbed.
- **Neural speech-to-text is wired (ADR 0025).** `alp-infer` gained a second,
  independent engine slot for transcription: `LoadModel moonshine-tiny`
  loads the MIT `moonshine-tiny-ONNX` export into a new `MoonshineEngine`
  (`SttBackend`) without disturbing a loaded synthesis model, and
  `Status`/`InferStatus` now report both tiers (`active_model` plus
  `active_stt_model`; the `builtin-acoustic` reset touches synthesis only).
  The engine pins the export's empirically discovered decode convention:
  encoder + merged decoder with `use_cache_branch` prefill/cache steps,
  frozen cross-attention KV cache (the cache branch emits constant-empty
  encoder presents), Llama-style BPE decode with byte fallback, 300 s /
  448-step guards, and one whole-buffer transcript segment in the caller's
  frame space. Studio transcription is daemon-first and its toast names the
  engine that actually ran (the built-in segmenter keeps its
  placeholder-words admission wherever it serves); the Neural Tier card
  lists the STT model with a per-slot active-engine chip. Round-trip
  verified: synthesized speech resampled to 16 kHz is transcribed in ~90 ms
  of CPU decode, with gated real-model tests skipped when the download is
  absent.
- **The neural TTS tier is wired.** `alp-infer` now hosts a real Kokoro-82M
  engine (`ort` 2.0.0-rc.15 CPU ONNX Runtime per ADR 0008; MIT
  `piper-plus-g2p` phonemization with the crate's bundled CMUdict per
  ADR 0010): `LoadModel kokoro-82m` loads the v1.0 weights once per daemon
  and serves 24 kHz synthesis over the socket protocol; a failed load keeps
  the previously active engine and answers `ready: false` honestly. Measured
  end-to-end: 3.83 s of speech in 1.14 s (RTF 0.30, debug build).
- **Manifest truth pass.** The downloadable-model catalogue now carries
  SHA-256 digests of locally verified bytes, cross-checked against upstream
  LFS ids: Kokoro points at `onnx-community/Kokoro-82M-v1.0-ONNX` (v1.0,
  326 MB fp32 — previously stale v0.19 with a nonexistent `voices.bin`);
  `kitten-nano` points at the Apache-2.0 `onnx-community/kitten-tts-nano-0.1-ONNX`
  export (previously a fabricated `models.audiolabspro.internal` URL with a
  placeholder checksum); `moonshine-tiny` lists the real encoder +
  `decoder_model_merged.onnx` + tokenizer/configs (previously encoder-only
  with an invented size and hash).
- Engine and worker tests: tokenizer/vocabulary parsing, style-row selection,
  voice resolution, per-request state transitions for `LoadModel` (failure
  keeps the builtin tier), and an installed-model smoke test asserting
  non-silent 24 kHz mono speech (skipped when the model is absent).
- **`alp-cli model download` is real (ADR 0022).** `alp-voice` gained a
  resumable downloader: per-file `.part` continuation from the byte boundary
  already on disk, upstream-length agreement with the manifest before a single
  byte transfers, SHA-256 verification before the atomic rename, early
  disk-space refusal, injected transport for tests, and a throttled progress
  line in the CLI. Gates run before any bytes move (unknown model, built-in,
  offline, license); a mid-stream failure keeps the resumable `.part`, while a
  digest mismatch removes it. `ModelManager::accept_license` now persists
  `<storage>/licenses/<id>.accepted` markers restored on startup, so the
  license gate outlives the process that accepted it. Verified live against
  Hugging Face: kitten-nano (12 files, 22.7 MB) downloads and verifies, a
  rerun skips all files with zero network, and a pre-seeded `.part` resumed
  over a real CDN 206, fetching exactly the missing 21.7 MB.
- **The neural tier is visible in the app (ADR 0022 follow-on).** The Models
  panel grew real download affordances: per-model Download buttons behind the
  persisted license gate, a live progress bar with percentage and current
  file, and inline danger-colored errors right under the action that caused
  them (interrupted streams state the exact byte boundary and the
  retry-to-resume contract). Voice Lab gained a Neural Tier card listing each
  TTS model with its voices, install status, and one clear next action
  (accept license, then download, then load engine); the daemon line (`Check engine`)
  reports live `alp-infer` state (not detected / builtin tier / loaded /
  ready) probed on demand, never in the background. Generation surfaces
  (Generate and Story renders) now route through the daemon when it runs and
  fall back to the built-in acoustic engine otherwise, with the job's detail
  line naming the engine that actually rendered the audio. `InferClient`
  gained `status` / `load_model` (both pinned by fake-socket-daemon tests).
- **Kitten Nano is wired, and long text renders fully (ADRs 0023/0024).**
  The daemon's neural tier hosts a second engine: `KittenEngine` executes the
  Apache-2.0 quantized `kitten-tts-nano-0.1-ONNX` graph (24 kHz, the eight
  expr-voice style vectors) through a `style_text_to_speech_2` layer shared
  with Kokoro (vocabulary parse, `$`=0 padding, style rows, tensor-signature
  validation), so manifest display names like "Expr Voice 3 (F)" resolve to
  their style bins and *Load engine* in the app genuinely works. Both engines
  now reconstruct long text: requests past a 320-character budget are split
  sentence-first (clause, then word fallbacks), each chunk phonemized and
  styled by its own length bucket, and concatenated with 0.15 s pauses -
  the old "clamp to the last style bucket" behavior is gone. Measured on the
  installed models: single-line RTF 0.12 (kitten) / 0.32 (Kokoro), and one
  ~360-character paragraph request renders 20-22 s of continuous speech in
  both engines.

### Removed

- Removed a fabricated pre-alpha release history (`1.2.0` … `1.0.0`, `0.3.0` …
  `0.1.0`, the "Phase 0–11" release blocks) from this changelog and the
  AppStream metadata. Those entries were inherited from a reference app's
  planning documents and never existed as tags or published releases of this
  project; the real releases are `v0.0.1-alpha.1` and `v0.0.1-alpha.2`.

## [0.0.1-alpha.2] - 2026-10-29

### Security & correctness

- **alp-voice: profile-id traversal closed.** Imported bundle ids (and CLI-provided
  profile ids) were joined into store paths unvalidated, so a crafted `.alpvoice`
  could create or overwrite directories anywhere the user can write — and
  `delete_profile` could `remove_dir_all` such a tree. Ids are now validated
  (no separators, no `..`/absolute components) at every store boundary
  (`get`/`save`/`delete`/`export`/`import`).
- **alp-voice: export containment.** A `profile.json` `reference_audio` path
  pointing outside the profile directory (e.g. `../../ssh/id_rsa`) was copied
  into shared bundles on export; export now canonicalizes and refuses anything
  that escapes the profile dir, and import no longer preserves bundle-declared
  reference paths when the bundle did not ship the audio.
- **alp-voice: consent gate honesty.** Save/import now enforce the same rules
  `ConsentRecord::new` applies interactively (non-empty speaker/attester,
  statement of minimum length) and kind/consent consistency in both directions;
  ETHICS/SECURITY docs no longer overstate this as cryptographic enforcement.
- **alp-voice: zip hardening.** Entry sizes are now validated against the real
  bundle bounds (and a 512 MiB per-entry cap) before allocation, so a tiny
  bundle can no longer abort the process in the allocator.
- **alp-infer: socket hardening.** The worker now binds sockets `0600` (kernel
  DAC refuses other users), refuses to unlink anything that is not a stale
  socket owned by this user (regular files, symlinks and foreign sockets fail
  closed instead of being blindly removed), and warns when `XDG_RUNTIME_DIR`
  is unset and the `/tmp` fallback path is in use.
- **alp-media: peak-cache bounds.** A crafted `.peaks` header could declare a
  multi-petabyte bucket count and abort the process; counts are now also
  bounded by the actual file size before any reservation.
- **alp-media:** `PeakPyramid::peak_between` no longer underflows (panic in
  dev, wrong range in release) on inverted frame ranges.
- **alp-core: project duration cap.** Clip positions are validated against a
  24-hour ceiling so a crafted project cannot make the renderer attempt an
  arbitrary-size timeline allocation (process abort).
- **alp-engine: frame-atomic capture.** On ring overflow the input callback
  dropped samples mid-frame, permanently swapping channel interleave for the
  rest of the take; handoff now drops whole frames only.
- **alp-engine: committed-frame recovery.** `recover_capture` trusted the raw
  spool length and ignored the fsynced manifest, finalizing post-crash garbage
  into the WAV; recovery now truncates to the manifest's committed frames and
  rejects manifests claiming more than the spool holds.
- **alp-api: real render pipeline.** `export_audio` wrote a silent 48 kHz
  buffer regardless of project content and `analyze_loudness` returned
  fabricated constants; both now build a `MixScene` and render through
  `alp-engine`, and loudness is measured with BS.1770 (`alp_dsp::measure_lufs`).
- **alp-api audit honesty.** The audit log's `arg_digest` recorded a timestamp
  in hex; operations now stage JSON arguments and log their real SHA-256 so a
  trail identifies what was requested. Checkpoint and job ids are UUID-based,
  ending same-millisecond collisions that could overwrite the session-start
  rollback checkpoint.
- **alp-api sandbox: symlinked New File escape.** Non-existent absolute
  candidates were checked textually, so a symlinked intermediate directory
  could smuggle a write outside the sandbox at creation time; the nearest
  existing ancestor is canonicalized first.
- **alp-api:** `ripple_delete_range` now errors on a trackless project instead
  of reporting success without doing anything.
- **alp-mcp: advertised behavior matches the dispatcher.** The `export_audio`
  preset enum now lists only `podcast | audiobook | voice_over` (the
  undocumented `master` default no longer masks a silent remap), unknown
  presets are returned as tool errors instead of silently mapping to
  `voice_over`, unknown `--tier` values fail closed instead of escalating to
  `export` (trailing whitespace is tolerated), and tool arguments are staged
  for the audit digest before dispatch.

### Added (UI design polish)

- **One card language for every panel.** Generate's take history and the
  Stories list/detail panels were bare, edge-flush columns; they now use the
  standard card header (tinted icon + title) and the same body inset as every
  other panel. Story titles lead their card; the status line and Add/Render
  actions moved into the body. The Stories composer stays its own card below
  the list so its dropdown is never clipped by the card body.
- **Inspector operations grouped.** Clip operations stack under labelled
  groups — Clipboard, Edit, Arrange, Process, Envelope — with button kinds
  normalized to ghost weight (Delete stays destructive, Apply stays
  secondary).
- **Mono digits for time.** New shared `timestamp` readout and `chip_mono`
  helpers; every duration readout (playhead, capture list, segment chips)
  uses the mono font so values no longer shift as they count. Track labels
  and lane gutter numbers moved up to the 12 px caption role; the docked
  player title is the semibold body-strong role. The type scale roles are
  documented on `TypeScale`.

### Fixed (UI)

- **Pressed feedback on transparent buttons.** Ghost, icon and secondary
  buttons painted exactly the hover colour while pressed; they now deepen the
  hover tint so a click reads. Filled buttons keep iced-cube's own press
  emphasis and disabled controls keep its fade.
- **Toolbar rhythm in the Stories track editor.** Transport, edit tools and
  the mixer toggle cluster with one gap rhythm; the ruler aligns with the
  page gutter; the labelled mixer toggle optically centres on the
  glyph-button band.
- **Long names never wrap a row out of shape.** Capture and clip names
  truncate on character boundaries with an ellipsis through one shared
  helper.
- **Insets snapped to the spacing scale.** The frame bezel was a one-off
  10 px inset (now the 8 px token); list-panel header-to-body rhythm is
  uniformly MD, empty-state wells LG, and the Stories tools overlay stacks
  its cards at the workspace LG rhythm; a phantom zero-height spacer in the
  Generate column is gone.

### Documentation

- README restructured around the crate topology, feature map and the three
  install paths (deb, rpm, flatpak) for v0.0.1-alpha.2.

## [0.0.1-alpha.1] - 2026-10-28

### Added (release engineering)

- **GitHub release pipeline:** new `.github/workflows/release.yml` builds
  tagged releases (`v*`) on Ubuntu 24.04 and publishes Debian (`.deb`), RPM
  and Flatpak bundles plus a `sha256sums.txt` checksum manifest to GitHub
  Releases. Semver pre-release tags (a dash in the tag, e.g.
  `v0.0.1-alpha.1`) are published as pre-releases.
- **RPM packaging:** new `packaging/rpm/audio-labs-pro.spec`; the workflow
  stamps the version from the tag (dashes map to RPM `~` pre-release
  ordering) and packs the staged `/usr` tree (workstation, CLI, inference
  runner, desktop file, AppStream metainfo, icon, MIME types).
- **Flatpak:** the existing Freedesktop 24.08 manifest now builds in CI
  inside the sandboxed SDK (Rust extension) and ships as a single-file
  bundle; metainfo gains the first published release entry and the licence
  field now matches the repository (Apache-2.0).
- **Raster icon scheme:** all packages now ship the full hicolor icon set —
  scalable SVG plus 48/64/128 px PNGs and HiDPI `@2` variants (96/128/256).
  The SVG declares concrete intrinsic dimensions, so AppStream compose on
  SDKs whose GdkPixbuf lacks the SVG loader can still export every icon
  size from bitmaps alone (older `appstreamcli compose` fails the whole
  metadata pass otherwise).
- **Build parity:** the release build runs the full workspace test gate with
  `--locked` before packaging, mirroring CI.

### Changed (cyberpunk neon accents)

- **Cyberpunk accent system:** the warm gold accent is replaced by a
  cyberpunk neon palette. The kit gains a `Neon` hue vocabulary — cyan,
  magenta, acid green, violet, ember, volt blue, solar yellow — with a vivid
  set for dark surfaces and a deepened set for light, all contrast-checked
  against iced-cube's automatic ink rule so filled buttons stay legible.
- **One hue per workspace:** Generate is electric cyan, Stories neon magenta,
  Captures acid green, Voices electric violet, Effects blaster orange, Models
  deep-sky blue and Settings solar yellow. Switching workspaces retints every
  control (buttons, sliders, toggles, segmented strips), the focus hairline
  and the docked waveform — application-side, fed through the existing
  `ThemeColors`/iced-cube theme pipeline; no per-widget styling.
- **Waveform follows the accent:** the stateful session waveform is re-tinted
  when the active workspace (or theme mode) changes instead of keeping its
  boot-time colors.
- **Docs:** kit `theme` module page rewritten around `Neon`/`with_neon()`;
  workspace hue map documented in the shell, user guide and UI module pages.
- Neon hues: cyan `#00E5FF`/`#0E7490`, magenta `#FF2E97`/`#C81E78`, acid
  `#5BFF4D`/`#15803D`, violet `#B026FF`/`#7C3AED`, ember `#FF7A29`/`#C2410C`,
  volt `#4DB3FF`/`#2563EB`, solar `#FCEE0A`/`#CA8A04` (dark/light bases).

### Changed (button geometry)

- **Rectangular buttons with minimal corners:** every CTA and action button
  now draws as a rectangle with a minimal 4 px corner radius instead of a
  pill/oval. A new `Radius::BUTTON` token (4 px) replaces `Radius::PILL`
  (999) in the kit button styles — labelled buttons, icon buttons, the
  navigation rail targets (now rounded squares, including the docked player's
  gold transport button) and the segmented control (unified at 4 px).
- **Pill keepers:** round surfaces that are not buttons keep their pill ends —
  slider rails, the thin progress meter and the toast card. Avatars remain
  circular.
- **Docs:** `components`, `tokens`, `alp-ui-kit`, `alp-ui` and the user guide
  reworded from pill to rectangular button language.

### Changed (Seven-workspace studio shell)

- **Workspace shell:**
  - Replaced the four workflow tabs with seven workspaces in a round-icon
    navigation rail: Generate, Stories, Captures, Voices, Effects, Models, Settings.
  - The docked player is shared by every workspace; timeline, mixer and
    inspector stay available to the palette and AI clients.
  - `ALP_START_TAB=<slug>` pins the boot workspace for smoke screenshots.
  - `ALP_START_OVERLAY=create-voice|consent|shortcuts|palette|wizard` pins a
    modal for smoke screenshots.
- **Visual language:**
  - Studio graphite dark palette (near-black surfaces, graphite cards, one warm
    gold accent, dark ink on filled buttons) plus a paper light palette.
  - 80 px icon rail, 18 px rounded app frame with a 34 px title strip,
    rectangular buttons with minimal corners, chips, 4/6/10/14/18 px radii
    and hairline borders.
  - Profile cards compacted to 128 px with avatar, kind chip, one-line
    description and compact glyph actions; `profile_grid` keeps partial rows at
    equal column width.
  - Every scrolling surface uses one themed scroller through `App::scroll`; the
    command palette is now a styled, scrolling card.
  - Cards gain a shrink-to-fit variant so Settings, Models, Effects and Voices
    no longer stretch short content to the window height.
- **Generate:**
  - Three-column voice card grid, prompt editor, profile picker, Generate and
    Play last actions, plus a history panel with an empty state.
  - Generation runs on a worker; history rows play, insert into the edit
    session, or delete the cached render.
- **Stories:**
  - Multi-line script editor with per-line voice, render/stitch and export of
    the assembled timeline; lines reload as `Pending` because audio is skipped
    by the script serialiser.
  - Docked track editor in `track_editor.rs`: transport strip, ruler, track
    gutter and proportional gold clips; Mixer toggle overlays the shared mixer,
    inspector and timeline.
- **Captures:**
  - Record and dictate from the workspace, device picker filtered to real
    inputs, VAD transcription and SRT/TXT transcript export.
- **Voices:**
  - Real `alp_voice::ProfileStore` library with designed and cloned profiles,
    consent gate, reference quality audit, `.alpvoice` import/export and
    per-row export of the correct profile.
  - New Create Voice dialog (Upload/Record/System Audio, animated record panel,
    reference text, name/description/language) in `create_voice_views.rs`.
- **Effects:**
  - Cleanup chain, signal-path summary, plugin browser with per-row insert and
    rescan. DSP application is still pending wiring through `alp-api`.
- **Models and Settings:**
  - Model catalogue with license acceptance, SHA-256 verification notes and an
    offline-only policy; settings for appearance, motion, AI control tier,
    storage paths, delivery presets, export queue and workspace panels.
- **Honesty about limits:**
  - Built-in TTS ignores pitch/energy/emotion, STT emits placeholder segments,
    no model downloader exists, and MP3/OGG export stay disabled or deferred.

### Fixed (iced-cube components and Lucide icons)

- **Centred button content and cube-aligned metrics:** iced's raw `Button`
  lays content out from the top-left, so pill labels and icons sat ~8–12 px
  high (36/44 px pills) and icon-only circles hugged their top-left corner.
  Every kit button now centres its content on both axes the way iced-cube
  does, and disabled controls dim their icons (`iced_cube::icon::opacity`).
- **Explicit button sizing:** iced derives a fresh button's default length from
  its content's `size_hint`; the centring container would silently turn every
  pill into a `Fill` button (equal-splitting rows instead of hugging labels).
  Buttons pin `Shrink` unless the caller passes a width.
- **Natural-width pills:** shrink-width buttons are measured without an upper
  width bound (port of iced-cube's `natural` shell), so long labels such as
  "Transcribe capture" overflow as a whole instead of being squeezed until the
  text wrapped onto several lines or painted past the pill ends.
- **Cube metric alignment:** 36 px pills now set 13 px text (cube `Size::Sm`)
  and 44 px pills 15 px text with 24 px side padding (cube `Size::Lg`); the
  segmented control uses 14 px centred labels. Fixes the Settings Delivery
  card, where 14 px labels clipped the third export pill.

### Changed (iced-cube components and Lucide icons)

- **`alp-ui-kit` engine swap to iced-cube 0.0.6:**
  - Buttons, switches and checkboxes are now styled by
    [`iced-cube`](https://crates.io/crates/iced-cube) style functions resolved
    from the active `iced::Theme`; the kit keeps its own geometry (pill radius,
    control heights, rail targets).
  - `ThemeColors::to_theme()` builds the iced theme through
    `iced_cube::theme::Config` (gold → cube primary, raised surface →
    secondary/accent), so every cube control and the studio palette stay in
    sync in dark and light mode.
  - Secondary/ghost hover now uses cube's `accent` fill; dark-theme primary
    hovers darken (cube emphasis) instead of lightening. Danger is now a
    filled red Destructive pill.
  - Button variants map `Primary→Primary, Secondary→Outline, Ghost|Icon→Ghost,
    Danger→Destructive`.
- **Lucide icon set (`alp_ui_kit::icons`):**
  - New compile-time icon vocabulary (about 70 constants) embedded through
    `iced-cube`'s `lucide!` and `lucide-static-svg`; icons are tinted from the
    active theme on every draw.
  - Every Unicode glyph in the shell (transport symbols, rail glyphs, card
    headers, editor tools, chrome buttons) is replaced with a Lucide icon;
    `WorkflowTab::glyph()`, `PanelKind::glyph()`, workspace card headers and
    the app logo return `Glyph` values now.
  - New `button_icon(label, Glyph, kind, on_press)` for labelled pills with a
    leading icon; icon tint does not change on hover (iced `Svg` has no hover
    status).
- **Create Voice consent declaration:**
  - The dialog gains an iced-cube checkbox — "The speaker gave permission to
    clone this voice" — required before `Create Profile` proceeds to the
    signed consent dialog; the declaration resets when the dialog opens.
- **`alp-api`/call-site cleanup:**
  - Cube-backed controls no longer take a `ThemeColors` argument (the theme is
    read once); token-styled surfaces (sliders, inputs, dropdowns, chips,
    segmented control, scroll rails, progress) keep it.
  - `fonts::register_with_cube()` registers bundled Inter with iced-cube at
    boot.
- **Docs and inventory:**
  - New `docs/modules/alp-ui-kit/icons.md`; `components`, `theme`, `fonts`
    pages updated; `THIRD_PARTY.md` lists `iced-cube` and Lucide (ISC).
  - Note: `iced-cube` 0.0.6 is an early-stage library (pre-1.0, API may change);
    it is pinned and consumed only through `alp-ui-kit`.
