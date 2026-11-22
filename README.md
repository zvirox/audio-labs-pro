<p align="center">
  <img src="packaging/branding/audio-labs-pro-logo.png" alt="Audio Labs Pro logo" width="300">
</p>

# Audio Labs Pro: Local Voice Studio & Multitrack Editor

[![CI](https://github.com/zvirox/audio-labs-pro/actions/workflows/ci.yml/badge.svg)](https://github.com/zvirox/audio-labs-pro/actions/workflows/ci.yml)
[![Version](https://img.shields.io/badge/Version-v0.0.1--alpha.5-success.svg)](https://github.com/zvirox/audio-labs-pro/releases)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Linux_(Wayland_&_X11)-orange.svg)](#4-minimum-requirements)
[![Rust](https://img.shields.io/badge/Rust-1.95+-informational.svg)](https://www.rust-lang.org)
[![UI](https://img.shields.io/badge/UI-iced_0.14_·_minimal_slate-indigo.svg)](docs/adr/0012-phase2-shell-and-glass.md)

A local, offline-by-default voice workstation for Linux: non-destructive
multitrack recording, editing, mixing and WAV/FLAC export, plus local speech
synthesis, dictation, voice profiles, consent-gated cloning and CLAP/LV2 plugin
hosting. Use the desktop shell, the headless CLI, or the 58-tool MCP server;
the inference worker runs out of process so model crashes cannot take down the
editor.

The shell ships **seven workspaces** — Generate, Stories, Captures, Voices,
Effects, Models and Settings. A compact labelled rail switches rooms, a docked
player stays available, and one restrained accent per workspace gives orientation
without turning the interface into a light show. The UI scales its grids, gutters
and dock controls to the window; a first-run wizard explains hardware, appearance
and motion before you start.

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
│ • Minimal slate UI  │ │ • new/import/render │ │ • 58 sandboxed tools    │
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
│  TTS · STT · clone models over ONNX Runtime & Candle │
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

| Workspace | Accent | What it holds |
|---|---|---|
| **Generate** | cyan | Voice cards, prompt editor, generate/play controls and take history |
| **Stories** | magenta | Multi-line scripts, per-line render, stitch, and the docked track editor |
| **Captures** | acid | Recording, dictation, VAD transcription, input meters, transcript editor |
| **Voices** | violet | Profile library, inspector, designed voices and the cloning consent gate |
| **Effects** | ember | Cleanup chain, signal path and hosted CLAP/LV2 plugins |
| **Models** | volt | Model catalogue, licences, offline policy and storage |
| **Settings** | solar | Appearance, motion, AI control, storage, delivery and panels |

One restrained hue per workspace: dark slate surfaces use lighter accents and
paper surfaces use deepened accents. Every text, muted-text and accent-ink pair
is contrast-checked in both themes, and the active rail target carries both a
fill and a text label so selection never depends on color alone. KWin/X11 can use
compositor blur; Wayland and GNOME keep a legible opaque tint
([ADR 0012](docs/adr/0012-phase2-shell-and-glass.md)).

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

- Local text-to-speech with text normalization and a pronunciation dictionary.
  The built-in acoustic tier works without a model download; optional neural
  TTS/STT models run in the isolated `alp-infer` worker so failures cannot take
  down the studio.
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

`alp-mcp` speaks JSON-RPC 2.0 (spec 2026-07-28) over stdio and exposes **58
tools** with workspace-jail sandboxing, path-traversal prevention, pagination,
limits and a JSONL audit trail. Its `--attach` mode proxies the same dispatcher
over an owner-only Unix socket into the running desktop: successful edits commit
to the one GUI undo history, history tools operate on that live stack, and
subscribed clients receive committed-change notifications.

Settings → AI control enables the socket with a fresh token, per-client human
approval, `SO_PEERCRED`, `0600` permissions and 2 MiB bounded frames. Cloning,
model installation, settings changes, shell execution and arbitrary file access
are outside MCP authority. Connection guides live in
[docs/mcp/CLIENT_SETUP.md](docs/mcp/CLIENT_SETUP.md); the security model in
[docs/mcp/SECURITY.md](docs/mcp/SECURITY.md), and honest remaining gaps in
[docs/mcp/AUDIT.md](docs/mcp/AUDIT.md).

---

## 3. Current status (v0.0.1-alpha.5)

- **Done:** P0 model tier; Phases 0–9 and 11; the seven-workspace shell; the
  58-tool headless MCP catalog; live attach with shared GUI undo, change
  subscriptions, revision/stale-read protection and measured request-to-commit
  latency; real Qwen3-TTS neural cloning with verified assets and consent
  enforcement.
- **Partial:** MCP `outputSchema`, word-level timestamp-dependent transcript
  cutting and filler removal, ~20 KB text response cap, multi-client flood
  soak, and rendering/pixel-presentation latency measurement.
- **Not shipped:** Streamable HTTP transport, cross-process project file locks,
  max-concurrent-job cap, and AI-only “undo all” (the UI offers one honest undo
  step). See [MASTER_CHECKLIST.md](MASTER_CHECKLIST.md) for item-by-item evidence.

---

## 4. Minimum Requirements

| Component | Minimum | Recommended |
|---|---|---|
| **OS** | Linux — X11 or Wayland | PipeWire session (PulseAudio/ALSA compatible) |
| **Architecture** | x86_64 | x86_64 (vectorized DSP paths) |
| **CPU** | 2 cores | 4+ cores for synthesis tiers |
| **RAM** | 2 GB (shell idle ≈ 86 MB RSS) | 8 GB for neural TTS/STT models |
| **Storage** | ~86 MB for the three stripped release binaries + assets | + model directory (per-model, see Models workspace) |
| **Audio** | ALSA device (48 kHz verified) | PipeWire / JACK-capable interface |
| **Build** | Rust 1.95+, `pkg-config`, ALSA headers, winit/iced graphics libs | `sccache`, cached `~/.cargo` |

The shell starts with a synthetic waveform, so it runs with no audio device and
no models — every dialog explains what it needs before touching hardware.

---

## 5. Installation

### Method 1: Packages from the release

The
[v0.0.1-alpha.5 release](https://github.com/zvirox/audio-labs-pro/releases)
ships `.deb`, `.rpm`, `.flatpak` and a `sha256sums.txt`.

```bash
# Verify checksums first
sha256sum -c sha256sums.txt   # (asset names may differ slightly from the manifest)

# Debian / Ubuntu
sudo apt install ./audio-labs-pro_0.0.1-alpha.5_amd64.deb

# Fedora / openSUSE (the RPM orders as a pre-release: 0.0.1~alpha.5)
sudo dnf install ./audio-labs-pro-0.0.1~alpha.5-1.x86_64.rpm

# Flatpak, any distribution (Freedesktop 25.08 runtime)
flatpak install --user ./Audio-Labs-Pro-0.0.1-alpha.5-x86_64.flatpak
```

The packages install `audio-labs-pro` (workstation), `alp-cli` and `alp-infer`,
with the desktop entry, AppStream metadata, icon scheme and MIME types
(`application/x-audio-labs-pro[-voice]`). `alp-mcp` is not packaged; use the
source build below if you need the MCP server.

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

Most project operations are also drivable without a display. Creating a cloned
voice profile and giving the required consent remain GUI-only by design
([Section 7](#7-the-command-line-tools)).

### Method 4: The MCP server

```bash
cargo build --release -p alp-mcp

# Headless: opens its own workspace
./target/release/alp-mcp --workspace /tmp/alp-workspace --tier export

# Attached: forwards to the running desktop (no workspace is opened)
# Enable Settings → AI control, approve the client, then copy its token.
ALP_CONTROL_TOKEN='TOKEN_FROM_THE_RUNNING_UI' \
  ./target/release/alp-mcp --attach --client-id my-local-client
```

Attach needs `XDG_RUNTIME_DIR`; the socket is `$XDG_RUNTIME_DIR/alp/control.sock`.
Client setup for Claude Desktop, Cursor and VS Code is in
[docs/mcp/CLIENT_SETUP.md](docs/mcp/CLIENT_SETUP.md).

### Configuration

| Variable | Default | Purpose |
|---|---|---|
| `ALP_CONFIG_HOME` | `$XDG_CONFIG_HOME/alp` | Workspace, theme and trusted-client settings |
| `ALP_DATA_HOME` | `$XDG_DATA_HOME/alp` | Projects, generated audio, voices and models |
| `ALP_CACHE_HOME` | `$XDG_CACHE_HOME/alp` | Peak and plugin caches |
| `ALP_CONTROL_TOKEN` | none | Fresh token from the running UI, required only for `alp-mcp --attach` |
| `ALP_START_TAB` | `generate` | Boot a named workspace for smoke tests |
| `ALP_FORCE_TINT` | unset | Skip compositor blur and use the opaque tint |
| `ALP_METRICS` | unset | Print frame timing to stderr |
| `ALP_OFFLINE` | unset | Force offline model downloads (`=1`) |

Model-specific overrides (`ALP_KITTEN_DIR`, `ALP_KOKORO_DIR`,
`ALP_MOONSHINE_DIR`, `ALP_QWEN3_DIR`) and the full list are documented in
[docs/README.md](docs/README.md) and [docs/mcp/CLIENT_SETUP.md](docs/mcp/CLIENT_SETUP.md).

---

## 6. Keyboard & Mouse

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

## 7. The Command-Line Tools

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
| `info` | Inspect a project, optionally as JSON |
| `lufs` / `cleanup` | Loudness measurement; voice cleanup chain |
| `tts` / `script render` | Speech from text or a JSON script |
| `model list` / `accept-license` / `download` / `verify` | Model catalogue, license gate, verified downloads |
| `profile` / `transcribe` / `vad` | Voice profiles, subtitles and speech segmentation |
| `hardware` | Report the local inference hardware profile |
| `plugin scan` / `list` / `inspect` | CLAP/LV2 discovery, registry and inspection |

`alp-infer` is the out-of-process inference worker the UI and API talk to; it
picks the local model tier (CPU/GPU) from the Models workspace. `alp-mcp` is the
automation server of [Method 4](#method-4-the-mcp-server). The acceptance
harnesses live beside them: `scripts/phase1-demo.sh` (peak cache, fixtures) and
`scripts/phase3-demo.sh` (import, autosave, export, null tests; `--record` for
forced-kill recovery, `--soak --minutes 30` for the dropout gate).

---

## 8. Testing & Verification

Pushes to `main` and pull requests run the full gate below in
[.github/workflows/ci.yml](.github/workflows/ci.yml). Tagged releases run the
locked tests, a release build and packaging in
[.github/workflows/release.yml](.github/workflows/release.yml):

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

## 9. FAQ

**Is this Linux-only?** Yes. Packages target x86_64 Linux; the shell supports
X11 and Wayland. Wayland uses an opaque tint rather than compositor blur because
the blur protocol needs unsafe code, which this project forbids.

**Does it work without audio hardware or models?** Yes. The shell starts with a
synthetic demo waveform and the acoustic TTS tier needs no download. Hardware
and model dialogs explain what is missing before they touch anything.

**Can an AI client clone my voice?** No. Creating a clone profile requires an
explicit human consent attestation in the GUI. MCP can design voices but cannot
create clones, install models, change settings, run commands or escape the
workspace.

**Which version am I running?** Release packages and tags use `v0.0.1-alpha.5`.
A source build reports workspace version `0.1.0` from `--version`; this is
intentional, not a different release.

**Why is `alp-mcp` missing after installing a package?** Packages ship
`audio-labs-pro`, `alp-cli` and `alp-infer`. Build `alp-mcp` from source for
headless or attached automation.

**Does it export MP3 or OGG?** No. WAV and FLAC are supported; MP3/OGG remain
out while their licensing is reviewed.

**What does attach latency measure?** Request-to-live-view-model commit time,
not actual pixels on screen. Expensive worker preparation is included in the
measurement; rendered-pixel latency is not claimed.

---

## 10. Troubleshooting

| Symptom | What to check |
|---|---|
| No window or a black canvas | Run with `ALP_FORCE_TINT=1`; confirm Mesa/WGPU libraries are installed and the session exposes X11 or Wayland. |
| No input or output device | `arecord -l` and `aplay -l`; add your user to `audio`; prefer a PipeWire session when available. |
| Clicks or xruns under PipeWire | Raise the quantum, for example `pw-metadata -n settings 0 clock.quantum 1024`. |
| A recording was interrupted | `alp-cli recover <file>` restores the committed frames from the recovery ledger. |
| A project edit was not saved | `alp-cli autosave <project>.alp` then `alp-cli recover-project <project>.alp`; the desktop also validates autosaves before restoring them. |
| A model download fails | Accept the model licence first with `alp-cli model accept-license <id>`, then retry; downloads are checksum-verified and stale `.part` files are kept for resume. |
| Neural synthesis is slow | Debug CPU inference is workload-dependent; the acoustic tier is instant and clearly labelled. Check `alp-cli hardware --json`. |
| Attach says “no socket” or authentication fails | Enable Settings → AI control, approve the client, copy the fresh token into `ALP_CONTROL_TOKEN`, and confirm `XDG_RUNTIME_DIR` is set. The token rotates on every enable; stale reads now fail with both `snapshotRevision` and `liveRevision`. |
| Wayland blur looks different | Expected: only KWin/X11 receives the blur hint; other sessions use the legible tint fallback. |

More detail is in [docs/TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md) and
[docs/mcp/CLIENT_SETUP.md](docs/mcp/CLIENT_SETUP.md).

---

## 11. Documentation

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

## 12. Legal

Audio Labs Pro is an independent open-source project. It is not affiliated with,
endorsed by or sponsored by any model vendor or streaming service. Model, codec
and plugin names are used only to describe compatibility and remain the
trademarks of their owners.

Everything runs locally. The only outbound traffic is a model download you
explicitly start from the Models workspace; there is no telemetry or account.
The optional AI control socket exists only while you enable it: it is owned by
your user, rejects other UIDs, requires a fresh token and human approval, and is
removed when you stop AI control. Model downloads are SHA-256 verified and each
model's licence is shown before you keep it.

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

## 13. License

Licensed under the **Apache License 2.0**. See the [LICENSE](LICENSE) file for
details. Third-party notices are collected in [THIRD_PARTY.md](THIRD_PARTY.md).
