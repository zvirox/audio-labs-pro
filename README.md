<p align="center">
  <img src="packaging/branding/audio-labs-pro-logo.png" alt="Audio Labs Pro logo" width="180">
  <h1 align="center">Audio Labs Pro</h1>
</p>

<p align="center">
  <strong>Local voice studio and multitrack audio workstation for Linux</strong>
</p>

<p align="center">
  <a href="https://github.com/zvirox/audio-labs-pro/releases"><img src="https://img.shields.io/badge/Release-Latest_Packages-blue.svg" alt="Releases" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-Apache_2.0-blue.svg" alt="License" /></a>
  <img src="https://img.shields.io/badge/Platform-Linux_(Wayland_&_X11)-orange.svg" alt="Platform" />
  <img src="https://img.shields.io/badge/Stack-Rust_·_Tauri_2_·_React_19-indigo.svg" alt="Stack" />
</p>

<p align="center">
  <a href="#what-is-audio-labs-pro">What is it?</a> ·
  <a href="#key-features">Key Features</a> ·
  <a href="#starship-control-integration">Starship Control Integration</a> ·
  <a href="#installation">Installation</a> ·
  <a href="#frequently-asked-questions">FAQ</a>
</p>

---

## What is Audio Labs Pro?

Audio Labs Pro is an offline-by-default voice workstation and multitrack audio editor built specifically for Linux. Whether you record podcasts, edit voiceovers, design dialogue for games, or clean up noisy room audio, it provides professional sound tools without requiring cloud subscriptions or sending your voice over the internet.

The application pairs a high-performance audio engine with a clean, distraction-free Linear-style dark theme. Speech generation and transcription models run in an isolated background process, ensuring that heavy AI calculations never interrupt smooth audio playback.

---

## Key Features

- **Non-Destructive Multitrack Editing**: Cut, trim, rearrange, and crossfade audio takes on an intuitive timeline. All original audio files remain untouched on disk.
- **Seven Dedicated Workspaces**: Seamlessly switch between Generate (speech synthesis), Stories (script narration), Captures (recording and dictation), Voices (profile manager), Effects (signal processing), Models (offline storage), and Settings.
- **Local Speech Synthesis (TTS)**: Generate clear, natural-sounding voiceovers directly from written text using private on-device models.
- **Automatic Dictation (STT)**: Transcribe speech with word-level accuracy. Use voice activity detection to capture spoken thoughts into searchable text notes.
- **Noise Cleanup & Signal Chain**: Remove electrical hums, HVAC rumble, and unwanted room noise with built-in cleanup filters and parametric equalization.
- **Audio Plugin Support**: Host standard CLAP and LV2 audio plugins directly inside your signal chain for custom mastering and effects.
- **High-Fidelity Audio Export**: Render your projects to standard uncompressed WAV or lossless FLAC files with custom sample rates and bit depths.

---

## Starship Control Integration

Audio Labs Pro functions entirely on its own as an independent desktop application. You do not need any additional software to record, edit, and export audio.

For an advanced agentic workflow, Audio Labs Pro links seamlessly with **Starship Control**, the unified Linux desktop AI assistant.

### Why It Exists

Post-production audio work often involves repetitive mechanical tasks, such as finding dead silences, filtering room noise, and stitching takes together. Connecting Audio Labs Pro with Starship Control allows you to direct these tasks using conversational voice or text prompts while you focus on creative decisions.

### Real-World Use Cases

- **Automated Audio Cleanup**: Ask Starship Control to *"Analyze track 1 for air conditioner hum and apply the appropriate high-pass filter."*
- **Script to Multitrack Narration**: Instruct the agent to *"Read this Markdown article, generate speech using my custom narrator profile, and lay the clips onto the timeline with 2-second spacing."*
- **Cross-App Video Production**: Request a full workflow: *"Take the audio from my latest Screen Studio Pro recording, remove background room noise in Audio Labs Pro, and export the clean take."*

### How Connection Works

Getting started with Starship Control is completely automatic:

1. **Automatic Discovery**: When Audio Labs Pro opens, it registers its presence in `$XDG_DATA_HOME/starship/plugins/alp.json`.
2. **Private Unix Bridge**: It establishes a dedicated local communication bridge at `$XDG_RUNTIME_DIR/starship/bridges/alp.sock`.
3. **Peer-Authenticated Security**: Connections are verified at the Linux kernel level using `SO_PEERCRED`. Only processes running as your logged-in user account can interact with the audio engine. No network sockets are exposed, and zero audio data leaves your machine.
4. **Immediate Control**: Summon Starship Control with `Super+Shift+Space`. It instantly recognizes Audio Labs Pro and stands ready to execute voice commands and workflow recipes.

---

## Installation

Prebuilt Linux packages are available on the [Releases page](https://github.com/zvirox/audio-labs-pro/releases).

### Debian, Ubuntu, and Pop!_OS (`.deb`)

```bash
sudo apt install ./audio-labs-pro_*_amd64.deb
```

### Fedora, RHEL, and openSUSE (`.rpm`)

```bash
sudo dnf install ./audio-labs-pro-*.x86_64.rpm
```

### Arch Linux and Manjaro (`.pkg.tar.zst`)

```bash
sudo pacman -U ./audio-labs-pro-*-x86_64.pkg.tar.zst
```

### Building From Source

Requirements: Rust stable, Node.js 22+, and development packages for ALSA (`libasound2-dev`), WebKitGTK (`libwebkit2gtk-4.1-dev`), and standard build tools.

```bash
git clone https://github.com/zvirox/audio-labs-pro.git
cd audio-labs-pro
npm install
npm run tauri build
```

---

## Frequently Asked Questions

#### Does Audio Labs Pro work completely offline?
Yes. Speech synthesis, dictation, audio processing, and project management run entirely locally on your computer. No internet connection is required after you download the app and your desired models.

#### Is Starship Control required to use Audio Labs Pro?
No. Audio Labs Pro is a complete, self-contained desktop workstation. Starship Control is an optional companion that provides cross-app coordination and natural language automation.

#### Can Audio Labs Pro handle large multi-hour recordings?
Yes. The 64-bit audio engine streams audio directly from disk, allowing you to record, edit, and mix multi-hour interviews and podcast sessions without running out of memory.

#### Are my custom voice profiles private?
Yes. Voice profiles and synthesized speech files are stored exclusively in your local application directory. They are never transmitted over the network or shared with external parties.

---

## License

Audio Labs Pro is open-source software licensed under the [Apache License, Version 2.0](LICENSE).
