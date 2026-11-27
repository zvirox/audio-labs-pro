# Audio Labs Pro — User Guide

Audio Labs Pro is a local-first Linux workstation for voice cloning, text-to-speech synthesis, multitrack nonlinear audio editing, spectral analysis, and AI-assisted MCP production.

---

## Installation (Linux)

Download the release package for your distribution from GitHub Releases (`packaging/nfpm.yaml`):

- **Debian / Ubuntu / Pop!_OS (`.deb`):**
  ```bash
  sudo apt install ./audio-labs-pro_*_amd64.deb
  ```
- **Fedora / RHEL / openSUSE (`.rpm`):**
  ```bash
  sudo dnf install ./audio-labs-pro-*.x86_64.rpm
  ```
- **Arch Linux / CachyOS / EndeavourOS (`.pkg.tar.zst`):**
  ```bash
  sudo pacman -U ./audio-labs-pro-*.pkg.tar.zst
  ```

Installed binaries:
- `/usr/bin/audio-labs-pro` — Desktop workstation (Tauri v2 + React Linear UI)
- `/usr/bin/alp-cli` — Headless CLI automation tool
- `/usr/bin/alp-infer` — Isolated local neural inference subprocess
- `/usr/bin/alp-mcp` — Standalone MCP JSON-RPC host binary

---

## Interface Tour (Linear UI)

Audio Labs Pro uses a high-contrast, opaque **Linear UI** without compositor blur or visual clutter:

- **Top TitleBar (40 px):** Displays the app mark, `Audio Labs Pro`, current version tag (from `tauri.conf.json`, e.g. `v0.1.0-beta.5`), drag region (double-click to maximize/restore), the global Light/Dark `ThemeToggle` (`Ctrl+D`), and window controls.
- **Collapsible Sidebar (`240 px` / `56 px`):** Switches between **Dashboard** (`Ctrl+0`) and 7 workspaces:
  1. **Voice Studio (`Ctrl+1`)** — Zero-shot voice cloning (requires written consent attestation), TTS synthesis, Neural Models, and Lexicon.
  2. **Multitrack DAW (`Ctrl+2`)** — Non-linear canvas arranger with discrete `Waveform | Spectrogram | Dual` per-track views, gain/pan envelopes, split/trim/slip editing, magnetic snap (`S`), and ripple edit (`R`).
  3. **Audiobook Lab (`Ctrl+3`)** — Multi-speaker narrative scripting and voice assignment.
  4. **Video Dubbing (`Ctrl+4`)** — Audio-sync workspace and speech generation workflows.
  5. **Batch & Repair (`Ctrl+5`)** — Live capture, VAD transcript preview, and noise repair tools.
  6. **DSP & Effects (`Ctrl+6`)** — Parametric EQ, dynamics, spatial reverb, and hosted CLAP / LV2 plugins.
  7. **Preferences (`Ctrl+7`)** — Audio hardware (PipeWire/ALSA), MCP AI control host tiers, window surface telemetry, and delivery loudness presets.
- **Breadcrumb & Inspector Bar:** Shows your current location (`Audio Labs Pro / Workspace / Subtab`), engine connection pill, revision counter, layout controls, and collapsible right **Inspector Drawer**.
- **Global Transport (`DockedPlayer`):** Bottom dock with Stop, Play/Pause (`Space`), Record Arm, Loop (`L`), timecode scrub bar, and discrete segmented L/R peak meters.
- **Command Palette (`Ctrl+K`):** Instant search across every workspace, floating studio panel, and theme action.
