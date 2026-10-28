# Changelog

All notable changes to Audio Labs Pro. Format loosely follows Keep a Changelog;
versions follow the release milestones in the build plan.

## [Unreleased]

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

## [1.2.0] - 2026-10-27

### Added (Phase 11 — CLAP & LV2 Audio Plugin Hosting, Release v1.2 "Plugins")

- **Decisions & Architecture:**
  - Resolved Decision in `docs/adr/0021-phase11-clap-and-lv2-plugin-hosting.md`.
  - Native Linux audio plugin hosting supporting modern open standards: CLAP 1.2+ (`clap.h`) and LV2 (`lv2.h`).
  - Safe Rust isolation boundaries around dynamic C ABI loading with zero unsafe code leaks outside `alp-plugin`.
  - Real-time audio callback safety: preallocated stereo scratch buffers and zero heap allocations inside the processing loop.
- **`alp-plugin` Hosting Engine:**
  - `clap`: Host descriptor negotiation, factory querying, stereo audio block routing, parameter querying, and thread-safe activation.
  - `lv2`: C ABI execution, Turtle manifest (`manifest.ttl`) parsing, dynamic `.so` discovery, audio port binding, and lifecycle controls.
  - `instance`: `ActivePlugin` providing unified interface for CLAP, LV2, and built-in virtual fallbacks with parameter normalization and JSON state serialization.
  - `scanner`: Filesystem discovery over standard paths (`~/.clap`, `/usr/lib/clap`, `~/.lv2`, `/usr/lib/lv2`, and `CLAP_PATH`/`LV2_PATH`), caching in `$XDG_CACHE_HOME/alp/plugins.json`.
  - `host`: `PluginHost` discovery coordinator and `TrackPluginChain` serial effect processor.
- **Engine & Project Integration:**
  - Integrated `TrackPluginChain` with `alp-engine`'s `Mixer` for real-time master and track bus processing.
  - Extended `alp-core`'s `EffectRef` with optional `state` payload to persist plugin settings in `.alp` project files.
- **CLI Commands (`alp-cli`):**
  - `alp-cli plugin scan`: Scan standard and custom directories for installed plugins and update cache.
  - `alp-cli plugin list`: List discovered plugins with format, category, vendor, and paths (text and JSON).
  - `alp-cli plugin inspect <id>`: Inspect plugin metadata and exposed automation parameters with value ranges and defaults.
- **Automation & MCP Tools (`alp-api` & `alp-mcp`):**
  - Added tools: `plugin_list`, `plugin_insert`, and `plugin_param_set`.
  - Direct parameter automation and plugin chaining via JSON-RPC.
- **Desktop UI Integration (`alp-ui`):**
  - Added "Hosted Plugins (CLAP & LV2)" card to the Effects workspace.
  - Rescan and one-click plugin insertion onto active tracks.
- **Fixed Layout Workspace Tabs (`alp-ui`):**
  - Replaced the floating window layout with four fixed workflow tabs: `Voice Studio`, `Voice Recording`, `Voice Editing`, and `Export Workflow`; the Unreleased shell supersedes this with seven workspaces.
  - Fixed-layout floating card design with subtle shadows and borders, removing closable/minimizable buttons.
  - Horizontal timeline anchored across the bottom of all editing workflows.
- **Documentation & Verification:**
  - New module documents in `docs/modules/alp-plugin/` and `docs/modules/alp-cli/commands/plugin.md`.
  - Acceptance demonstration script `scripts/phase11-demo.sh` verifying end-to-end plugin scanning, hosting, CLI, and MCP automation.

## [1.1.0] - 2026-10-27

### Added (Phase 10 — MCP: AI-driven editing, creation, and export, Release v1.1 "Agent")

- **Decisions & Architecture:**
  - Resolved Decisions D9–D11 in `docs/adr/0020-phase10-mcp-architecture-and-protocols.md`.
  - Implemented Model Context Protocol (MCP) spec revision `2026-07-28` support (stateless, `initialize`, `server/discover`, `tools/list`, `tools/call`, `resources/list`, `resources/read`, `prompts/list`, `prompts/get`).
  - Strict architectural decoupling: `alp-mcp` connects exclusively through `alp-api`, forbidding direct links to audio rendering, DSP, or media codecs.
- **`alp-api` Automation Layer:**
  - Typed operations over audio domain crates (`alp-core`, `alp-media`, `alp-dsp`, `alp-voice`, `alp-engine`).
  - Three-tier permission model: `Observe` < `Edit` < `Export` with runtime capability enforcement.
  - Filesystem sandboxing (`WorkspaceSandbox`) enforcing canonical directory allowlists and blocking path traversal or symlink escapes.
  - Automatic session checkpointing (`CheckpointManager`) enabling one-click full rollback to session start ("Undo all AI changes").
  - Persistent JSONL audit logging (`AuditLogger`) tracking timestamps, client IDs, tool names, argument hashes, and revision deltas.
  - Asynchronous background job manager (`JobManager`) with progress tracking and cooperative cancellation.
- **`alp-mcp` Server Binary:**
  - Standalone MCP server executable operating over standard input/output (`stdio`) for headless AI agents.
  - Typed JSON-RPC 2.0 dispatching engine with schema reflection, structured content, and actionable error messages.
  - Registered core tool catalog: `project_info`, `project_save`, `history_undo`, `history_redo`, `checkpoint_create`, `checkpoint_restore`, `rollback_session`, `track_list`, `track_add`, `track_remove`, `clip_split`, `range_delete`, `analyze_loudness`, `export_audio`.
  - Built-in prompt recipes: `clean_up_voice_recording`, `produce_podcast_episode`, `narrate_script`, `remove_fillers_and_dead_air`.
  - Served resources under `alp://docs/agent-guide` and `alp://project/current`.
- **Desktop UI Integration (`alp-ui`):**
  - Added `AiActivity` panel with master switch, permission tier selector, and live activity audit feed.
  - Chrome status indicator badge (`⚡ AI Active` / `⚡ AI`) with tooltip and one-click toggle.
  - Emergency safety controls: **Stop AI** kill switch and **Undo all AI changes** rollback button.
- **Documentation & Verification:**
  - Complete MCP documentation in `docs/mcp/`: `TOOLS.md`, `CLIENT_SETUP.md`, `AGENT_GUIDE.md`, `WORKFLOWS.md`, `SECURITY.md`.
  - 12 new module documentation files across `docs/modules/alp-api/`, `docs/modules/alp-mcp/`, and `docs/modules/alp-ui/app/`.
  - Automated executable acceptance script `scripts/phase10-demo.sh`.

## [1.0.0] - 2026-10-27

### Added (Phase 9 — polish, packaging, and 1.0)

- **Packaging & Desktop Integration:**
  - Resolved Decision D12: registered canonical application ID `io.github.zvirox.AudioLabsPro`.
  - Desktop Entry specification: `packaging/desktop/io.github.zvirox.AudioLabsPro.desktop` with categories, file associations, and desktop actions.
  - AppStream 1.0 metainfo specification: `packaging/desktop/io.github.zvirox.AudioLabsPro.metainfo.xml` with OARS-1.1 content ratings and release history.
  - MIME type registration: `packaging/mime/io.github.zvirox.AudioLabsPro.xml` for `application/x-audio-labs-pro` (`*.alp`) and `application/x-audio-labs-pro-voice` (`*.alpvoice`).
  - High-resolution scalable application vector icon: `packaging/icons/hicolor/scalable/apps/io.github.zvirox.AudioLabsPro.svg`.
  - Standalone AppImage build pipeline: `scripts/build-appimage.sh` constructing integrated AppDir and `AppRun` launcher.
  - Flatpak manifest: `packaging/flatpak/io.github.zvirox.AudioLabsPro.yaml` configuring Wayland, PipeWire, ALSA, and GPU device permissions.
  - Debian package specification: `packaging/deb/control`.
- **Reliability & Stress Testing:**
  - Added comprehensive test suite `crates/alp-core/tests/reliability_and_stress.rs`:
    - Fuzzing & corrupt JSON rejection: invalid schema versions, missing fields, cyclic IDs, and path traversal protection in media paths.
    - Autosave recovery: corrupted autosaves are safely held for inspection without replacing clean project files, and valid autosaves restore interrupted work.
    - Stress benchmarks: projects with 50 tracks, 500 clips, and 500 undo steps serialize and deserialize in under 30 ms.
- **Accessibility & Usability:**
  - First-Run Setup Wizard: interactive calibration modal configuring audio inputs/outputs, system hardware tiering, appearance theme, and glass blur effects.
  - Keyboard Shortcuts Reference Modal (`F1`): instant on-screen cheat sheet for playback, navigation, editing, and dictation hotkeys.
- **Documentation:**
  - Comprehensive User Guide: `docs/USER_GUIDE.md`.
  - Keyboard Shortcuts cheat sheet: `docs/KEYBOARD_SHORTCUTS.md`.
  - Voice Cloning Ethics & Safety Policy: `docs/ETHICS.md`.
  - Platform & Hardware Troubleshooting Guide: `docs/TROUBLESHOOTING.md`.
  - Updated license inventory in `THIRD_PARTY.md` and module docs.
  - Automated `scripts/phase9-demo.sh` acceptance harness.

## [0.3.0] - 2026-10-26

### Added (Phase 8 — voice profiles, cloning, and voice generation, Releases v0.2 "Voice" and v0.3 "Clone")

- `alp-voice`:
  - `profile`: persistent `ProfileStore` managing profiles in `$XDG_DATA_HOME/alp/voices`, prebuilt profiles ("Aura", "Atlas", "Echo", "Sol"), and `VoiceParams` parameter tuning.
  - `zip`: pure-Rust zero-dependency ZIP archive writer and reader (method 0 STORED + CRC-32) for portable `.alpvoice` archive import and export.
  - `ConsentRecord`: enforced human consent gate requiring speaker name, attester identity, and written permission statement; cloning without signed consent is strictly rejected at the library level.
  - `reference`: `analyze_reference_quality` assessing duration ($\ge 5.0$ s), digital clipping spikes, noise floor, and silence ratio with actionable advice.
  - `design`: `design_voice` synthesizing customized voice profiles from natural language briefs.
  - `cache`: cryptographic `VoiceGenerationCache` deduplicating identical synthesis requests via SHA-256.
  - `compare`: `compare_audio` providing side-by-side A/B acoustic similarity metrics.
  - `hardware`: `HardwareProfile` assessing CPU cores, RAM, and GPU presence, recommending "Fast" vs "Best" model tiers.
- `alp-infer`:
  - Handles `InferRequest::DesignVoice` and `InferResponse::VoiceDesigned`.
- `alp-cli`:
  - `alp-cli profile list / info / check-reference / design / export / import / compare / delete`: full headless profile lifecycle management.
  - `alp-cli hardware`: hardware capability probing and model tier recommendations.
- `alp-ui`:
  - Voice Lab updated with profile selector, reference audio audit button with status readouts, and human consent attestation card.
  - Voice design natural language prompt input.
  - Models panel updated with hardware capability and tiering recommendations.
- ADR 0018, Phase 8 results, module docs, and automated `scripts/phase8-demo.sh` acceptance harness.

### Added (Phase 7 — dictation and transcription)

- `alp-voice`:
  - `vad`: streaming and batch Voice Activity Detection (`VoiceActivityDetector`, `segment_speech`) using dual-criterion energy RMS thresholding and zero-crossing rate windowing with hangover buffering and speech onset backdating.
  - `transcript`: `Transcript` document model owning frame-accurate `TranscriptSegment` items; export to TXT, SubRip SRT (`00:00:01,234`), and WebVTT (`00:00:01.234`); sample-rate aware SRT parsing (`parse_srt`); timeline cut mapping (`cut_range_for_segment`) for text-based audio ripple editing.
  - `stt`: streaming `DictationSession` emitting `DictationEvent` (`SpeechStarted`, `Partial`, `Final`, `Silence`) for live microphone input with sub-second perceived latency; `BuiltinSttBackend` baseline acoustic transcription.
  - `protocol` & `client`: IPC messages `InferRequest::Transcribe` and `InferResponse::Transcript` added to `alp-infer` daemon protocol, with transparent in-process STT fallback.
  - `model`: registered `moonshine-tiny` manifest in standard catalogue.
- `alp-infer`:
  - Handles `InferRequest::Transcribe` for out-of-process speech transcription.
- `alp-cli`:
  - `alp-cli transcribe <input> [--out <output>] [--format txt|srt|vtt] [--json]`: transcribe audio into text or subtitles.
  - `alp-cli vad <input> [--threshold-db <float>] [--hangover-ms <int>] [--min-speech-ms <int>] [--json]`: segment audio into speech intervals and report vocal percentage.
- `alp-ui`:
  - Added Voice & Dictation Lab view with live microphone dictation toggle and realtime feedback preview.
  - Added transcript segment browser with click-to-seek playhead positioning.
  - Text-based editing: one-click "×" button on transcript segments executes undoable ripple deletions on the timeline.
  - Transport bar updated with dedicated dictation toggle button.
  - Models panel updated with Moonshine Tiny STT entry.
- ADR 0017, Phase 7 results, module docs, and automated `scripts/phase7-demo.sh` acceptance harness.

### Added (Phase 6 — model manager and built-in TTS)

- `alp-voice`:
  - `ModelManager`: catalogue-driven model registry, per-model licensing agreements, XDG directory resolution, atomic `.part` download commitment, and offline mode.
  - `sha256`: self-contained FIPS 180-4 standard cryptographic hash implementation for model integrity verification.
  - `TextNormalizer`: rule-based spoken word expansion for currency, numbers, ordinals, dates, percentages, and abbreviations.
  - `PronunciationDictionary`: custom phonetic respelling overrides and pause (`[pause=500ms]`)/emphasis markup parsing.
  - `Script` & `ScriptLine`: multi-line narration data model with single-line dirty invalidation, gapless stitching, and batch stem export.
  - `protocol`: length-prefixed JSON framing over Unix domain sockets (`InferRequest`, `InferResponse`).
  - `client`: `InferClient` connecting to `alp-infer` daemon with seamless in-process `BuiltinAcousticBackend` fallback generating 48 kHz formant speech without external dependencies.
- `alp-infer`:
  - Out-of-process daemon executable isolating neural inference and model memory from the editing application.
  - Handles `Ping`, `GetStatus`, `LoadModel`, `Synthesize`, `Cancel`, and `Shutdown`.
- `alp-cli`:
  - `alp-cli tts <text> <output.wav> [--voice <id>] [--speed <float>]` for command-line speech generation.
  - `alp-cli model list / verify / accept-license` for model lifecycle management.
  - `alp-cli script render <script.json> <output> [--stitch]` for batch script production.
- `alp-ui`:
  - Voice Lab panel updated with active voice selector, live text input, and line synthesis.
  - Models panel updated with actual catalogue status.
- ADR 0016, Phase 6 results, module docs, and automated `scripts/phase6-demo.sh` acceptance harness.

### Added (Phase 5 — effects and voice cleanup, Release v0.1 "Editor")

- `alp-dsp`:
  - Direct Form biquad filters implementing standard RBJ cookbook curves (HighPass, LowPass, Peaking, LowShelf, HighShelf, Notch, BandPass).
  - 5-band voice-optimized `ParametricEq` (HPF 80Hz, LowShelf 200Hz, Bell 800Hz, Bell 3.5kHz, HighShelf 10kHz).
  - Soft-knee `Compressor` with attack/release envelopes and makeup gain.
  - Downward `NoiseGate` with threshold, attack, hold, and release ballistics.
  - Peak `Limiter` guaranteeing strictly bounded output ceilings without clipping.
  - Vocal cleanup processors: `DeEsser` (sidechain bandpass detector), `DeHum` (50Hz/60Hz mains notch cascade with 4 harmonics), and `DeClick` (transient derivative spike detector and polynomial repair).
  - ITU-R BS.1770-4 K-weighting filter, integrated LUFS loudness measurement (`measure_lufs`), and target normalization calculator (`calculate_normalization_gain`).
  - Composable `EffectChain` and one-click `VoiceCleanup` preset pipeline.
- `alp-core`: track effect list model (`Track.effects: Vec<EffectRef>`) and undoable `SetTrackEffects` command.
- `alp-cli`:
  - `alp-cli lufs <file>`: measures integrated LUFS, peak linear/dBFS, and computes normalization gain targets for Podcast, Streaming, and Broadcast.
  - `alp-cli cleanup <input> <output> [--mains 50|60]`: executes full vocal cleanup pipeline offline.
- `alp-ui`:
  - Effects panel wired with active vocal processing chain (De-Hum, De-Click, Voice EQ, De-Esser, Compressor, Peak Limiter).
  - Per-effect enable/bypass toggles, parameter inspector readouts, and one-click "Voice Cleanup Preset" button.
  - Integrated loudness target reference (-16.0 LUFS Podcast standard / -1.0 dBFS ceiling).
- ADR 0015, Phase 5 results, module docs, and automated `scripts/phase5-demo.sh` acceptance harness.

### Added (Phase 4 — multitrack timeline and mixing)

- `alp-core`: multitrack project model with per-track gain, pan, mute, solo, arming,
  track colors, and master bus gain. Named markers, regions, and loop ranges with
  undoable commands.
- `alp-engine`: multitrack mix scene with standard stereo balance pan law, master bus
  gain with limiter, live meter streaming via `play_scene_live`, and 10-track scene
  rendering with 0.00 null error.
- `alp-media`: `ExportPreset` enum with standard configurations for `Podcast` (44.1 kHz
  16-bit FLAC), `Audiobook` (44.1 kHz 16-bit WAV), and `VoiceOver` (48.0 kHz 24-bit WAV),
  with triangular TPDF dither.
- `alp-cli`: `render --preset` and `render-stems --preset` supporting delivery targets,
  plus stem rendering of individual tracks as WAV or FLAC.
- `alp-ui`: timeline rendering of live tracks, markers and loop range, configurable
  crossfade inspector actions (0.5s, 1.0s), and direct preset buttons in the Export Queue.
  Modular refactoring of `session` into `operations`, `export`, `import`, and `tests`.
- ADR 0014, Phase 4 results, and updated `scripts/phase4-demo.sh` acceptance harness.

### Added (Phase 3 — recording and single-track editing)

- `alp-core`: selection/cursor/snap model, clipboard fragments, undoable
  `DeleteRange`/`SilenceRange`/`PasteClips`/`DuplicateClip`/`PunchIn` commands,
  `NormalizeClip`, and non-destructive reverse/invert clip flags.
- `alp-core`: source-level peak measurement and serde defaults, plus atomic
  `autosave/project.json` recovery through `ProjectPaths`.
- `alp-engine`: crash-safe capture to a raw f32 spool with a chunk-synced JSON
  manifest, `recover_capture`, peak/RMS input meter, device selection and
  optional monitoring. Reverse and inverted clips now render in mixer and
  offline render.
- `alp-media`: WAV/FLAC export with bit depth, sample rate, deterministic TPDF
  dither, RIFF/Vorbis metadata, atomic replacement, and a null-test comparison
  helper. FLAC works around Symphonia 0.6.1's short-final-block rejection.
- `alp-cli`: `render --format/--bit-depth/--sample-rate/--dither` plus tags,
  import-time resampling/conversion and peak capture, `record --device/--monitor/
  --chunk-seconds`, `recover`, `delete-range`, `autosave`, `recover-project` and
  `null-test`.
- `alp-ui`: input picker, monitor and punch-in toggles, record start/stop with
  peak/RMS meters, drag/drop background import, a live `EditSession` inspector
  with clip list, selection fields, cut/copy/paste/delete/silence/trim/split/
  duplicate, normalize/reverse/invert, envelope dip/reset, gain/fades, undo/redo,
  recording import and background mix export.
- Phase 3 module docs, ADR 0013, `scripts/phase3-demo.sh` and results page.

### Changed

- `alp-ui/src/app.rs` was split: panel rendering now lives in
  `alp-ui/src/app/views.rs`; the live editing model lives in
  `alp-ui/src/session.rs`.
- `flacenc` is now a Phase 3 dependency (`default-features = false`).
- `alp-engine`: the capture ring buffer is ten seconds and the spool writer polls
  at 1 ms, so chunk syncing no longer drops callbacks on a busy disk.

### Measured (Phase 3)

- `scripts/phase3-demo.sh`: 2-clip 5 s project, 44.1 kHz → 48 kHz conversion,
  autosave recovery from 3 to 4 clips, 480,000-frame reference, 24-bit WAV and
  FLAC exports, null-test max difference 1.8e-7 (threshold 0.001).
- 30-minute stereo capture soak: 86,401,536 frames, 1,800.032 s, 168,753
  callbacks, 0 dropped samples.
- Forced-kill capture recovery: 379,904 frames recovered after `SIGKILL`.
- Workspace tests: 144 passing plus doctests; fmt and clippy `-D warnings` green.

### Known gaps (Phase 3)

- Hardware acceptance was run on the system default 48 kHz stereo input; other
  devices, sample rates and channel counts remain untested.
- A 30-minute stereo capture needs about 1.5 GB free for the spool and final
  WAV; the harness workspace follows `TMPDIR`.
- The desktop session opens a fixed `ui-demo` project and autosaves it, but
  there is no project picker, save-as or recent-project list.
- Import after recording is synchronous and can briefly block the UI.
- Normalization uses the source peak, not a per-clip measured peak.
- Monitoring is a nearest-frame copy, not a filtered resampler.

### Added (Phase 2 — shell and glass system)

- `alp-ui-kit`: design tokens, bundled Inter variable font (family
  `Inter Variable`) with OFL text, dark/light glass palettes, semantic
  buttons/icon buttons/toggles/sliders/dropdowns/tabs/tooltips/progress, cards,
  modal, toast, command palette, panel motion and a peak-pyramid canvas
  waveform.
- `alp-ui`: transparent undecorated window, custom chrome, four visible panel
  defaults among eight stubs, drag/resize/z-order/snap/collapse/dock, responsive
  geometry, versioned atomic `workspace.json`, command palette with autofocus,
  keyboard shortcuts, Reduce motion, theme cycling, toast, worker job queue and
  demo peaks.
- Glass probe using iced 0.14's safe `window::run` handles; KWin/X11
  `_KDE_NET_WM_BLUR_BEHIND_REGION` through `x11rb`; honest tint fallback with
  `ALP_FORCE_TINT`.
- `scripts/check-ui.sh` guard against raw `iced::widget::button` in `alp-ui`,
  wired into CI with the Linux GUI build packages.
- Phase 2 module docs, results, ADR 0012, updated architecture/stack/README and
  third-party inventory.

### Changed

- ADR 0003 is superseded: no vendored `iced_winit` is needed because iced 0.14
  exposes raw handles safely; ADR 0012 records the KWin/X11 path and the
  unsafe Wayland limit.
- ADR 0007 now points at the UI-widget guard and records that panel motion uses
  a lilt ease-out offset, with a spring settle deferred.
- CI installs winit/iced graphics libraries in addition to ALSA headers.

### Measured (Phase 2)

- 1-hour (3,600 s) waveform autoscroll: 60–61 frames/s, 16.6 ms average
  frame time; first second 45–55 frames/s during load.
- Idle CPU after startup: 0.00% over 7 s (0 scheduler ticks); idle RSS about
  86 MB. One early run sampled a transient 4.25% during startup rendering.
- 104 workspace tests pass (30 `alp-ui`, 18 `alp-ui-kit`, plus the Phase 1
  suites and one doctest).

### Known gaps (Phase 2)

- Cross-desktop KDE/GNOME matrix not run on this machine; GNOME has no supported
  blur protocol in this build. The KWin/X11 path compiles and its eligibility is
  unit-tested; the compositor effect still needs a manual KDE/X11 run.
- The panels are stubs; transport/timeline do not drive the engine or real media.
- No full accessibility tree, screen-reader labels or in-app text scale.
- `app.rs` is close to the 1,500-line ceiling; split views before the next UI
  feature.
- No standalone popover/spinner; button hover states are immediate, not spring
  animated.

### Added (Phase 1 — foundation)

- Cargo workspace (edition 2024, Rust 1.89) with nine crates:
  `alp-core`, `alp-media`, `alp-engine`, `alp-dsp`, `alp-voice`, `alp-infer`,
  `alp-ui-kit`, `alp-ui`, `alp-cli`.
- `alp-core`: typed UUID ids, project/source/track/clip model, validation,
  `FrameRange` helpers, all Phase 1 undo/redo commands, versioned JSON schema,
  atomic saves, XDG app directories and logging setup.
- `alp-media`: symphonia decode/probe/stream, rubato FFT resampling,
  `PcmBuffer`, 16/24-bit and float WAV output, streaming peak builder, and the
  `ALPPEAKS` three-level disk cache with hit/miss detection.
- `alp-engine`: immutable `MixScene`, sample-accurate `Mixer` with gain, fades
  and envelope automation, `Transport`, lock-free engine command channel, cpal
  output/input, offline mixdown and WAV render.
- `alp-cli`: `gen`, `decode`, `peaks`, `new`, `import`, `info`, `render`,
  `play`, `devices`, `record`, with JSON output where useful.
- Guard scripts for dependency directions, file length and module docs.
- GitHub Actions CI (Ubuntu 24.04, ALSA headers, fmt, clippy `-D warnings`,
  tests, build, all guards).
- `scripts/phase1-demo.sh` acceptance harness (1-hour peaks, eight-clip render,
  optional playback/soak/capture).
- ADR 0011 (Phase 1 workspace and real-time seam); module docs for every crate
  and module.

### Changed

- ADR 0004/0009 corrected to match the implemented cache fingerprint and the
  absence of an automated allocator guard.
- Architecture, stack, README and license inventory advanced to Phase 1.

### Measured

- 1-hour 48 kHz mono WAV: peak cache miss 1.42–1.54 s, hit 0.013–0.015 s.
- Eight simultaneous 60 s clips: 2,880,000 rendered frames in 1.5 s, no
  clipping with the demo’s low-amplitude source.
- 10 s default-device playback: 931 callbacks, 476,672 frames, no stream error.
- 10-minute eight-clip soak: 56,244 callbacks, 28,796,928 frames, no stream
  error (see `docs/spikes/PHASE1-RESULTS.md`).

### Added (Phase 0 — research spikes)

- Phase 0 spike workspace (`spikes/`): iced glass/waveform demo, cpal
  playback/capture harness, symphonia + peak-cache benchmark, Kokoro ONNX
  inference, Wayland blur-protocol probe.
- Documentation: architecture draft, ADRs 0001–0010, spike results with
  measurements, third-party license inventory.
- Measured: 60 fps scrolling of a 1-hour waveform, 0–1% idle CPU, 10.66 ms audio
  callback period with no gaps, 1-hour decode+peaks in 998 ms, Kokoro RTF
  0.26–0.36 with an MIT G2P front-end.

### Decisions

- Apache-2.0, Linux-only, English-only.
- iced 0.14.0 pinned; in-house `alp-ui-kit`; lilt + in-house spring.
- WAV/FLAC/OGG Vorbis at v1; MP3 deferred (LGPL).
- Moonshine for dictation; `alp-infer` as a separate process; heavy tier
  deferred to Phase 8.
- Blur path: KWin/X11 property via safe `x11rb`; Wayland/GNOME tint fallback
  (ADR 0012 supersedes the ADR 0003 vendor patch).
- Heavy voice-cloning tier deferred to Phase 8 (ADR 0008).

### Known gaps

- Cross-desktop matrix (KDE, GNOME, X11, NVIDIA) not run.
- Moonshine and heavy-model spikes not run.
