# Audio Labs Pro: Local Voice Studio & Multitrack Editor

[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/Version-v0.0.1--alpha.1-success.svg)](https://github.com/zvirox/audio-labs-pro/releases)
[![Platform](https://img.shields.io/badge/Platform-Linux_(Wayland_&_X11)-orange.svg)](#3-minimum-requirements)
[![Rust](https://img.shields.io/badge/Rust-1.89+-informational.svg)](https://www.rust-lang.org)
[![UI](https://img.shields.io/badge/UI-iced_0.14_·_frosted_glass-indigo.svg)](docs/adr/0012-phase2-shell-and-glass.md)
[![Workspaces](https://img.shields.io/badge/Workspaces-7_selectable_rooms-brightgreen.svg)](#the-seven-workspaces)

A Linux audio studio written in Rust: multitrack recording, editing, mixing and
export, plus local AI text-to-speech, dictation, voice profiles and CLAP/LV2
plugin hosting. Fully offline — the only network traffic is user-initiated model
downloads.

The shell ships **seven workspaces** — Generate, Stories, Captures, Voices,
Effects, Models and Settings — each carried by its own cyberpunk neon hue and
switched live from the icon rail. Every one of them is the same studio: same
motion, same geometry, same icons, same navigation, one docked player.

---

## 1. System Topology

```
┌────────────────────────────────────────────────────────────────────────┐
│                         AUDIO LABS PRO TOPOLOGY                        │
└────────────────────────────────────────────────────────────────────────┘
                                 │
     ┌───────────────────────────┼───────────────────────────┐
     ▼                           ▼                           ▼
┌─────────────────────┐ ┌─────────────────────┐ ┌─────────────────────────┐
│       ALP-UI        │ │       ALP-CLI       │ │         ALP-MCP         │
│   desktop shell     │ │   headless tools    │ │  AI automation server   │
├─────────────────────┤ ├─────────────────────┤ ├─────────────────────────┤
│ • Seven workspaces  │ │ • gen/decode/peaks  │ │ • JSON-RPC 2.0 (stdio)  │
│ • Frosted glass     │ │ • new/import/render │ │ • 14 sandboxed tools    │
│ • GPU waveform      │ │ • play/record/recover│ │ • Session rollback +   │
│ • Command palette   │ │ • voices/dictation  │ │   JSONL audit trail     │
│ • iced 0.14 + kit   │ │ • plugin scan/list  │ │ • Client sandboxing     │
└──────────┬──────────┘ └──────────┬──────────┘ └────────────┬────────────┘
           │                       │                         │
           ▼                       ▼                         ▼
┌──────────────────────────────────────────────────────┐ ┌────────────────┐
│        ALP-ENGINE (in-process audio thread)          │ │    ALP-API     │
│  cpal callbacks · mixer · transport · recorder       │ │ automation API │
│  Real-time safe: no allocation, no locks, no logging │ └────────────────┘
└──────────────────────────┬───────────────────────────┘
                           │ versioned Unix-socket protocol
                           ▼
┌──────────────────────────────────────────────────────┐
│          ALP-INFER (separate process)                │
│  TTS · STT · clone models over ONNX Runtime & GGUF   │
│  Crash-isolated: an OOM or GPU fault never kills     │
│  the editor.                                         │
└──────────────────────────────────────────────────────┘
  Shared libraries: ALP-CORE (project model, undo) · ALP-DSP (effects) ·
  ALP-MEDIA (decode/encode/peaks) · ALP-VOICE (profiles, consent, protocol) ·
  ALP-PLUGIN (CLAP/LV2 hosting) · ALP-UI-KIT (tokens, Inter, motion)
```

Dependency directions are enforced by a guard
([`check-deps.sh`](scripts/check-deps.sh)): `alp-core` depends on no other ALP
crate, the engine never reaches the DSP stack mid-callback, and the UI never
touches cpal or ONNX. See
[Architecture](docs/architecture/ARCHITECTURE.md).

---

## 2. Features

### The seven workspaces

An icon rail on the left switches rooms; a player is docked at the bottom of
every screen (Stories replaces it with its track editor).

| Workspace | Neon | What it holds |
|---|---|---|
| **Generate** | cyan | Voice cards, prompt editor, generate/play controls and take history |
| **Stories** | magenta | Multi-line scripts, per-line render, stitch, and the docked track editor |
| **Captures** | acid | Recording, dictation, VAD transcription, input meters, transcript editor |
| **Voices** | violet | Profile library, inspector, designed voices and the cloning consent gate |
| **Effects** | ember | Cleanup chain, signal path and hosted CLAP/LV2 plugins |
| **Models** | volt | Model catalogue, licences, offline policy and storage |
| **Settings** | solar | Appearance, motion, AI control, storage, delivery and panels |

One hue per workspace: the palette derives vivid neon bases for dark surfaces
and deepened bases for light, both contrast-checked so filled controls stay
legible under the kit's automatic ink rule. The surfaces stay frosted glass —
KWin on X11 gets real compositor blur; Wayland and GNOME keep the legible tint
fallback ([ADR 0012](docs/adr/0012-phase2-shell-and-glass.md)).

### Multitrack timeline & editing

- Sample-accurate multitrack editing: split, crossfades, volume envelopes,
  ripple deletes (R), clipboard cut/copy/paste, punch-in, trim, fade in/out,
  gain, normalize, reverse and invert — everything undoable (Ctrl+Z/Y) through
  a real command stack, with autosave recovery behind it.
- Mixer with pan, master bus and delivery presets (podcast, audiobook,
  voice-over), markers and a loop range, per-stem and full renders.
- Import takes any WAV/FLAC/MP3/OGG source and converts it to the session
  format (44.1→48 kHz verified in the acceptance harness).

### Captures: recording & recovery

- Crash-safe capture with recovery ledgers; a 30-minute 48 kHz stereo soak
  recorded 86,401,536 frames with **0 dropped samples**, and a capture killed
  with `SIGKILL` recovered 379,904 frames — see
  [Phase 3 results](docs/spikes/PHASE3-RESULTS.md).
- Live input peak/RMS metering, monitoring, chunked recording and a recover
  command (`alp-cli record`, `alp-cli recover`).

### Generate & Voice Lab

- Built-in neural TTS with text normalizer and a pronunciation dictionary;
  synthesis runs in the isolated `alp-infer` worker so model failures can never
  take down the studio.
- Voice profiles — prebuilt **Aura**, **Atlas**, **Echo**, **Sol** — plus `.alpvoice`
  bundles you can import by dropping them on the window. Cloning is behind an
  enforced human consent attestation, with reference-audio quality auditing,
  prompt-based voice design, a generation cache and A/B comparison.
- **Stories** renders scripts line by line (dirty-line regeneration on edit),
  stitches takes, and edits them on the docked track editor.

### Captures: dictation & transcription

- Dual-threshold VAD speech segmentation and a streaming dictation session with
  sub-second feedback (Ctrl+Space).
- Transcript document model with text-based ripple cutting and exports to
  **TXT / SRT / WebVTT** (`Ctrl+Shift+T` transcribes the active track).

### Effects & plugins

- Real-time chain: high-pass, 5-band voice EQ, compressor, noise gate, peak
  limiter, de-esser, de-hum, de-click — plus ITU-R BS.1770 LUFS metering and
  one-click voice cleanup.
- **CLAP & LV2 hosting** through the `alp-plugin` engine: dynamic C-ABI loading,
  a scanner with a cached registry, real-time-safe scratch buffers, and the
  Hosted Plugins card in the Effects workspace.

### Export & delivery

- WAV and FLAC export with deterministic TPDF dither (triangular) and RIFF /
  Vorbis tags; the null-test difference is 1.8e-7 at 24-bit — bit-transparent.
- MP3/OGG export is intentionally absent while their licensing picture is
  resolved; WAV/FLAC cover production today.
- AI clients share the same capability through the MCP `export_audio` tool.

### AI automation (MCP)

`alp-mcp` speaks JSON-RPC 2.0 (spec 2026-07-28) over stdio: 14 tools from
`project_info` to `rollback_session`, directory-jail sandboxing, path-traversal
prevention, and a persistent JSONL audit trail. Connection guides for Claude
Desktop, Cursor and VS Code live in
[docs/mcp/CLIENT_SETUP.md](docs/mcp/CLIENT_SETUP.md); the security model in
[docs/mcp/SECURITY.md](docs/mcp/SECURITY.md).

---

## 3. Minimum Requirements

| Component | Minimum | Recommended |
|---|---|---|
| **OS** | Linux — X11 or Wayland | PipeWire session (PulseAudio/ALSA compatible) |
| **Architecture** | x86_64 | x86_64 (vectorized DSP paths) |
| **CPU** | 2 cores | 4+ cores for synthesis tiers |
| **RAM** | 2 GB (shell idle ≈ 86 MB RSS) | 8 GB for neural TTS/STT models |
| **Storage** | ~30 MB installed (release binary + assets) | + model directory (per-model, see Models workspace) |
| **Audio** | ALSA device (48 kHz verified) | PipeWire / JACK-capable interface |
| **Build** | Rust 1.89+, `pkg-config`, ALSA headers, winit/iced graphics libs | `sccache`, cached `~/.cargo` |

The shell starts with a synthetic waveform, so it runs with no audio device and
no models — every dialog explains what it needs before touching hardware.

---

## 4. Installation

### Method 1: Packages from the release

The
[v0.0.1-alpha.1 release](https://github.com/zvirox/audio-labs-pro/releases)
ships `.deb`, `.rpm`, `.flatpak` and a `sha256sums.txt`.

```bash
# Verify checksums first
sha256sum -c sha256sums.txt   # (asset names may differ slightly from the manifest)

# Debian / Ubuntu
sudo apt install ./audio-labs-pro_0.0.1-alpha.1_amd64.deb

# Fedora / openSUSE (the RPM orders as a pre-release: 0.0.1~alpha.1)
sudo dnf install ./audio-labs-pro-0.0.1~alpha.1-1.x86_64.rpm

# Flatpak, any distribution (Freedesktop 24.08 runtime)
flatpak install --user ./Audio-Labs-Pro-0.0.1-alpha.1-x86_64.flatpak
```

The packages install `audio-labs-pro` (workstation), `alp-cli` and `alp-infer`,
with the desktop entry, AppStream metadata, icon scheme (SVG + 48/64/128 px and
HiDPI PNGs) and MIME types (`application/x-audio-labs-pro[-voice]`).

### Method 2: Build from source

```bash
git clone https://github.com/zvirox/audio-labs-pro.git
cd audio-labs-pro

# Ubuntu is the reference dependency list (CI installs exactly this)
sudo apt-get install libasound2-dev pkg-config \
  libxkbcommon-dev libwayland-dev wayland-protocols \
  libx11-dev libxcursor-dev libxrandr-dev libxi-dev \
  libgl1-mesa-dev libegl1-mesa-dev libfontconfig1-dev \
  libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev

cargo build --release
cargo run --release -p alp-ui        # the desktop shell
```

`ALP_START_TAB=stories` boots a different workspace; `ALP_FORCE_TINT=1` disables
the KWin blur request; `ALP_METRICS=1` prints frame metrics.

### Method 3: Headless CLI

```bash
cargo run -p alp-cli -- new demo.alp --name "My Project"
cargo run -p alp-cli -- import demo.alp demo.wav
cargo run -p alp-cli -- render demo.alp mix.flac --format flac \
  --bit-depth int24 --dither triangular --title "My mix"
```

Everything the UI does — voices, dictation, plugins, MCP tooling — is also
drivable without a display (see [Method 4](#6-the-command-line-tools)).

### Method 4: The MCP server

```bash
cargo build -p alp-mcp
# then point your client at the stdio binary; see docs/mcp/CLIENT_SETUP.md
```

---

## 5. Keyboard & Mouse

| Shortcut | Action |
|---|---|
| **Space / Home / Esc** | Play–pause / rewind / stop or dismiss |
| **Ctrl+K** | Command palette (fuzzy search over every action) |
| **F1** | Shortcuts help modal |
| **Ctrl+S / Ctrl+O** | Save project (atomic) / import audio |
| **Ctrl+D / Ctrl+Shift+M** | Cycle theme / toggle reduced motion |
| **Ctrl+1 … Ctrl+8** | Jump to panel |
| **Ctrl+Z / Ctrl+Y** | Undo / redo |
| **Ctrl+B** | Split clip at playhead |
| **Delete / Ctrl+X / Ctrl+C / Ctrl+V** | Delete / cut / copy / paste range |
| **R** | Toggle ripple editing |
| **Ctrl+Space / Ctrl+Shift+T** | Toggle live dictation / transcribe track |
| **Ctrl+Enter** | Synthesize the current dialogue line |

The full sheet is [`docs/KEYBOARD_SHORTCUTS.md`](docs/KEYBOARD_SHORTCUTS.md);
the same modal opens inside the app with F1.

---

## 6. The Command-Line Tools

`alp-cli` exercises the whole engine without a display, and every subcommand is
script-safe (non-zero exit on failure):

| Command | What it does |
|---|---|
| `gen` | Generate a test WAV (seconds, channels) |
| `decode` / `peaks` | Inspect a file; compute and cache peak data |
| `new` / `import` | Create a project; convert and import media into it |
| `render` | Render a project to WAV/FLAC with dither and tags |
| `play` | Play a project through the engine |
| `devices` | List ALSA capture/playback devices |
| `record` / `recover` | Chunked capture with recovery ledger; restore after a kill |
| `autosave` / `recover-project` | Snapshot edits; restore the last snapshot |
| `null-test` | Bit-transparency check between reference and rendered audio |
| `plugin scan` / `list` / `inspect` | CLAP/LV2 discovery, registry and inspection |

`alp-infer` is the out-of-process inference worker the UI and API talk to; it
picks the local model tier (CPU/GPU) from the Models workspace. `alp-mcp` is the
automation server of [Method 4](#method-4-the-mcp-server). The acceptance
harnesses live beside them: `scripts/phase1-demo.sh` (peak cache, fixtures) and
`scripts/phase3-demo.sh` (import, autosave, export, null tests; `--record` for
forced-kill recovery, `--soak --minutes 30` for the dropout gate).

---

## 7. Testing & Verification

Every push runs the same gate as a release:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
bash scripts/check-file-length.sh   # hard 1,500 / soft 1,000 lines per file
bash scripts/check-deps.sh          # architecture dependency directions
bash scripts/check-ui.sh            # no raw iced widgets outside the kit
bash scripts/check-docs.sh          # every module has its required doc sections
```

Hardware acceptance evidence and per-phase numbers (waves in
[docs/spikes/](docs/spikes/)) reproduce with the demo harnesses above; the
AppStream-valid metainfo and package layouts are exercised on every tagged
release by [.github/workflows/release.yml](.github/workflows/release.yml).

---

## 8. Documentation

- [`docs/README.md`](docs/README.md) — the index: architecture, stack, module docs, ADRs.
- [`docs/USER_GUIDE.md`](docs/USER_GUIDE.md) — the studio manual, workspace by workspace.
- [`docs/KEYBOARD_SHORTCUTS.md`](docs/KEYBOARD_SHORTCUTS.md) — hotkey cheat sheet.
- [`docs/architecture/ARCHITECTURE.md`](docs/architecture/ARCHITECTURE.md) — crates, processes,
  threading and enforced dependency directions.
- [`docs/spikes/`](docs/spikes/) — per-phase acceptance measurements (Phase 0 → 11).
- [`docs/adr/`](docs/adr/) — architecture decision records.
- [`docs/mcp/`](docs/mcp/) — tool catalog, client setup, agent guide, workflows, security.
- [`docs/ETHICS.md`](docs/ETHICS.md) — cloning ethics and the consent gate.
- [`docs/TROUBLESHOOTING.md`](docs/TROUBLESHOOTING.md) — Wayland, NVIDIA, PipeWire and ALSA setup.
- [`docs/modules/`](docs/modules/) — one required-sections page per crate and module.
- [`THIRD_PARTY.md`](THIRD_PARTY.md) — every crate, model and asset with its license.
- [`CHANGELOG.md`](CHANGELOG.md) — release history.

---

## 9. Legal

Audio Labs Pro is an independent open-source project. It is not affiliated with,
endorsed by or sponsored by any model vendor or streaming service. Model, codec
and plugin names are used only to describe compatibility and remain the
trademarks of their owners.

Everything runs locally. The only network traffic is a model download you
explicitly start from the Models workspace; there is no telemetry, no account
and no background listener. Model downloads are SHA-256 verified and the
licensing of each model is shown before you keep it.

Voice cloning is gated: creating a clone profile requires an explicit human
consent attestation that you are the speaker or have the speaker's permission,
and `.alpvoice` bundles carry that declaration with them. See
[docs/ETHICS.md](docs/ETHICS.md) for the full policy.

CLAP and LV2 hosting loads only plugins you have installed on your own machine,
through the loaders those ecosystems define — nothing else is scanned or
executed.

Everything here is provided as is, without warranty of any kind, under the
Apache-2.0 licence below.

---

## 10. License

Licensed under the **Apache License 2.0**. See the [LICENSE](LICENSE) file for
details. Third-party notices are collected in [THIRD_PARTY.md](THIRD_PARTY.md).
