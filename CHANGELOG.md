# Changelog

All notable changes to Audio Lab Pro. Format loosely follows Keep a Changelog;
versions follow the release milestones in the build plan.

## [Unreleased]

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
