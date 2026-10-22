# Changelog

All notable changes to Audio Lab Pro. Format loosely follows Keep a Changelog;
versions follow the release milestones in the build plan.

## [Unreleased]

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
- Blur path: vendor a minimal `iced_winit` patch to expose window handles
  (ADR 0003); tint fallback ships until it lands.
- Heavy voice-cloning tier deferred to Phase 8 (ADR 0008).

### Known gaps

- Cross-desktop matrix (KDE, GNOME, X11, NVIDIA) not run.
- Moonshine and heavy-model spikes not run.
