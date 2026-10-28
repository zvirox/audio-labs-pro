# 0014 — Multitrack timeline, stem rendering and delivery presets

- Status: accepted
- Date: 2026-10-27
- Phase: 4 (multitrack timeline and mixing)

## Context

Phase 4 expands the single-track workspace into a multitrack timeline with mixing,
stems export, crossfades, markers, and delivery presets. The engine must support
arbitrary track counts, pan laws, per-track mute/solo/arm, master bus gain, and stem
routing without compromising real-time safety.

## Decision

### Multitrack Mixing and Routing

1. Tracks own gain (`0.0..=2.0`), stereo pan (`-1.0..=1.0`), mute, solo, arming and
   display color. Pan follows a standard balance law, attenuating the opposite channel.
2. Master bus applies `master_gain` (`0.0..=4.0`) across all active tracks and is capped
   by a peak limiter stage.
3. Stem rendering (`render-stems`) renders each track through its channel strip as a
   standalone 24-bit WAV or FLAC file with sanitized file slugs.
4. Armed overdub import: newly recorded or imported takes land across all currently armed
   tracks simultaneously, or default to the primary track when none are armed.

### Timeline Editing and Crossfades

1. Crossfade is an explicit undoable command (`CrossfadeClips`) that overlaps adjacent
   clips by adjusting their timeline bounds and applying linear fade curves.
2. The UI inspector exposes multiple crossfade durations (0.5s, 1.0s) and track movement
   actions (`MoveSelectedClipToNextTrack`, `MoveSelectedClipToPrevTrack`).
3. Named markers (`Marker`) and timeline loop ranges (`FrameRange`) are stored in the
   project model and reflected in the timeline ruler and transport.

### Export Presets

1. `ExportPreset` introduces industry targets:
   - `Podcast`: 44.1 kHz, 16-bit FLAC, triangular TPDF dither.
   - `Audiobook`: 44.1 kHz, 16-bit WAV, triangular TPDF dither.
   - `VoiceOver`: 48.0 kHz, 24-bit WAV, triangular TPDF dither.
2. Presets are available from the CLI (`--preset`) and the UI Export Queue panel.

## Consequences

- Stems export matches the full mix null-test within agreed thresholds.
- No file in `alp-ui` exceeds 700 lines following the modular session split.
- Acceptance demo `phase4-demo.sh` verifies stems, master mix, and preset renders.
