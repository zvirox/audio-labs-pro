# 0019 — Polish, Packaging, and Release Standards

- Status: accepted
- Date: 2026-10-27
- Phase: 9 (Polish, packaging, and 1.0)

## Context

Phase 9 transitions Audio Labs Pro from feature-complete releases into a production-grade workstation that users can easily install, configure, trust, and operate reliably across Linux distributions. This requires resolving Decision D12 (canonical reverse-DNS application ID), establishing standardized Linux desktop integration (Desktop Entry, AppStream 1.0 metadata, shared MIME definitions, and scalable vector branding), authoring reproducible packaging pipelines (AppImage, Flatpak, Debian), hardening engine reliability against corrupted project files and interrupted crashes, introducing an accessible first-run setup wizard and on-screen keyboard shortcut reference, and conducting a comprehensive license and documentation audit.

## Decision

### 1. Reverse-DNS Application ID (Decision D12)
- Anchored the application ID to the developer's project namespace: `io.github.zvirox.AudioLabsPro`.
- Standardized binary name as `audio-labs-pro` for the graphical workstation, `alp-cli` for headless operations, and `alp-infer` for out-of-process neural inference daemon.

### 2. Desktop Integration and MIME Registration
- Authored XDG Desktop Entry at `packaging/desktop/io.github.zvirox.AudioLabsPro.desktop` registering categories (`AudioVideo;Audio;AudioVideoEditing;Recorder;`), file associations, and desktop actions.
- Authored AppStream 1.0 metainfo specification at `packaging/desktop/io.github.zvirox.AudioLabsPro.metainfo.xml` with OARS-1.1 content rating and release history.
- Authored shared MIME specification at `packaging/mime/io.github.zvirox.AudioLabsPro.xml` defining:
  - `application/x-audio-labs-pro` (`*.alp`)
  - `application/x-audio-labs-pro-voice` (`*.alpvoice`)
- Designed and bundled high-resolution scalable vector icon at `packaging/icons/hicolor/scalable/apps/io.github.zvirox.AudioLabsPro.svg`.

### 3. Packaging Pipelines
- **AppImage:** Authored standalone assembly script `scripts/build-appimage.sh` producing self-contained `AppDir` with `AppRun` dynamic launcher script and bundled metadata.
- **Flatpak:** Authored modern Flatpak manifest `packaging/flatpak/io.github.zvirox.AudioLabsPro.yaml` targeting `org.freedesktop.Platform` 24.08 runtime with permissions for Wayland, fallback X11, PipeWire, ALSA devices, and DRI GPU acceleration.
- **Debian:** Authored Debian package control specification in `packaging/deb/control`.

### 4. Reliability, Fuzzing, and Crash Recovery
- Created automated test harness `crates/alp-core/tests/reliability_and_stress.rs`:
  - Verified fuzzed and malformed JSON inputs (empty, invalid schema versions, missing envelope, out-of-range sample rates, cyclic clip/track IDs) are rejected safely without panicking.
  - Verified path traversal protection (`resolve_relative` blocks `..` and absolute escapes).
  - Verified autosave recovery: corrupted autosaves are safely held for inspection without replacing clean project files, while valid autosaves restore interrupted session work.
  - Benchmarked large projects (50 tracks, 500 clips, 500 undo steps): serialization and deserialization execute under 30 ms.

### 5. Accessibility, Usability, and First-Run Setup
- Added `first_run_completed` flag to `PersistedWorkspace`.
- Implemented `first_run_wizard_view` in `alp-ui::wizard_views` displaying:
  1. Audio hardware device detection and active recording channel status.
  2. Hardware capability probe (RAM, CPU cores, GPU render nodes) and recommended model tier.
  3. Appearance theme and compositor glass / tint fallback preview.
- Implemented `shortcuts_help_modal_view` in `alp-ui::wizard_views` triggered by `F1` or Command Palette, providing instant on-screen shortcut assistance.

### 6. Documentation and Governance
- Authored end-to-end `docs/USER_GUIDE.md`.
- Authored `docs/KEYBOARD_SHORTCUTS.md`.
- Authored `docs/ETHICS.md` detailing ethical voice cloning and acceptable use.
- Authored `docs/TROUBLESHOOTING.md` covering Wayland, NVIDIA, PipeWire, and ALSA buffer tuning.
- Completed license audit in `THIRD_PARTY.md` verifying 100% permissively licensed dependencies.

## Consequences

Audio Labs Pro meets all Phase 9 milestone criteria: reproducible release packaging, crash-resilient project recovery, fluid accessible user interface, and complete platform documentation.
