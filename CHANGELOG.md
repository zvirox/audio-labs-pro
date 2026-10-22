# Changelog

All notable changes to Audio Labs Pro. Format loosely follows Keep a Changelog;
versions follow the release milestones in the build plan.

## [Unreleased]

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
