# Audio Labs Pro — User Guide

Audio Labs Pro is a studio-styled, local-first voice audio workstation built in
Rust. It combines non-destructive single-track and multitrack editing with local
speech synthesis, voice profiles, transcription, speech-cleanup DSP and an
undoable session engine. Nothing leaves the machine.

---

## 1. Getting Started

### Installation
- **AppImage:** Run `./Audio-Labs-Pro-x86_64.AppImage` directly on any modern Linux distribution.
- **Flatpak:** Install using `flatpak install io.github.zvirox.AudioLabsPro.flatpak`.
- **Source Build:** Run `cargo build --release --workspace` and execute `target/release/alp-ui`.

### First-Run Wizard
On first launch, Audio Labs Pro displays an interactive setup wizard:
1. **Audio Hardware Setup:** Probes your system's ALSA / PipeWire devices and selects the active recording input.
2. **System Capabilities:** Analyzes RAM, CPU core count, and GPU/DRM render nodes to recommend the optimal AI model tier ("Fast" lightweight vs "Best" high-fidelity).
3. **Appearance & Compositor Glass:** Tests your compositor for background blur capabilities and enables hardware-accelerated glass or pristine tinted glass fallback.

---

## 2. The Seven Workspaces

An icon rail on the left switches between workspaces. A player is docked at
the bottom of every screen; Stories replaces it with a track editor.

| Icon | Workspace | What it holds |
|---|---|---|
| ✨ Sparkles | Generate | Voice cards, prompt editor, generate/play controls and take history |
| 🎬 Clapperboard | Stories | Multi-line scripts, per-line render, stitch and the docked track editor |
| 💿 Disc | Captures | Recording, dictation, VAD transcription, input meters and transcript |
| 👥 Users | Voices | Profile library, inspector, designed voices and the cloning gate |
| ⚡ Zap | Effects | Cleanup chain, signal path and hosted CLAP/LV2 plugins |
| 📦 Boxes | Models | Model catalogue, licences, offline policy and storage |
| ⚙ Gear | Settings | Appearance, motion, AI control, storage, delivery and panels |

The **timeline**, **mixer** and **inspector** remain available to the palette and
AI clients. In Stories, the **Mixer** button opens them over the line cards; close
it with the same button. Ripple delete lives in the mixer; recording and
dictation live in Captures.

**Create Voice** opens a modal with Upload, Record and System Audio tabs. Record
captures up to 30 seconds. **Create Profile** first asks you to tick the
speaker-permission declaration, then presents the human consent attestation.
Upload and System Audio state plainly that they are not wired in this build.
**Import Voice** accepts `.alpvoice` bundles dropped on the window.

---

## 3. Multitrack Timeline & Editing

### Non-Destructive Editing
Audio Labs Pro preserves source recordings immutably. Every edit produces non-destructive timeline clips that reference source ranges.

- **Split Clip (`Ctrl+B`):** Splits the active clip at the current playhead position.
- **Crossfading:** Overlapping adjacent clips creates smooth equal-power crossfades.
- **Volume Envelope & Fades:** Adjust per-clip gain, linear/logarithmic fade-ins, and fade-outs in the Inspector panel.
- **Ripple Mode (`R`):** When ripple mode is active, deleting or cutting a clip or transcript segment automatically pulls subsequent clips left, eliminating dead air.
- **Infinite Undo / Redo (`Ctrl+Z` / `Ctrl+Y`):** Every timeline operation, effect tweak, voice insertion and generated take is fully undoable.

---

## 4. Captures: Recording & Dictation

1. Open the **Captures** workspace.
2. Select your input device from the dropdown. Placeholder ALSA "null" devices are hidden.
3. Choose **Record** or **Dictate**.
4. Audio streams with lock-free atomic meters and is flushed to disk in crash-safe chunks every 5 seconds. If power is interrupted, autosave recovery restores your project on reopening.

Stopping a recording adds a row to the capture list and runs VAD segmentation.
Transcripts export as **SRT** or plain **TXT** from the same workspace. The
built-in recogniser currently writes placeholder segment text: timings and
ripple cuts are real, the words are a stand-in.

---

## 5. Cleanup DSP Chain

Audio Labs Pro ships a mastering-grade voice processing chain:

- **High-Pass Filter (HPF):** Eliminates low-end rumble (selectable 40 Hz to 120 Hz).
- **De-Hum:** Adaptive notch filter cascade targeting 50 Hz and 60 Hz electrical mains hum and their first 4 harmonics.
- **De-Click:** Fast sample-rate transient detector that isolates and interpolates micro-clicks and mouth pops.
- **Parametric EQ:** 5-band equalizer with gentle presence boosts and resonance cuts.
- **De-Esser:** Dynamic gain-reduction band focused around 5.5 kHz – 7.5 kHz to soften harsh sibilance.
- **Vocal Compressor:** Soft-knee downward compressor with program-dependent attack/release for consistent voice levels.
- **Peak Limiter:** True-peak lookahead limiter set to -1.0 dBFS ceiling to prevent digital clipping.

The chain is applied by the CLI and export render paths. The **Effects**
workspace shows and edits the chain and hosts CLAP/LV2 plugins, but the desktop
preview currently plays unprocessed audio.

---

## 6. Generate: Synthesis & Scripts

The **Generate** workspace turns text into speech locally:

- **Text-to-Speech:** Enter text, pick a profile (*Aura*, *Atlas*, *Echo*, *Sol* or your own), choose a tier and press **Generate**.
- **Render History:** Each take is cached on disk. Rows can play, insert into the
  timeline or be deleted.
- **Stories:** The **Stories** workspace keeps multi-line scripts as JSON with a
  voice per line. **Render story** generates pending lines on a worker and
  **stitch** assembles them into one take; script audio is not persisted, so lines
  return as Pending after a restart.

The built-in acoustic backend is a deterministic local synth; it does not yet
honour pitch, energy or emotion hints.

---

## 7. Voice Profiles & Cloning

### Reference Audio Quality Gate
Before creating a cloned voice profile, reference audio must pass an objective acoustic audit:
- Duration: $\ge 5.0$ seconds.
- Clipping: Zero detected digital clipping.
- Noise Floor: $\le -35$ dBFS.
- Silence Ratio: $\le 60\%$ speech activity.

### Human Consent Attestation Gate
In strict compliance with ethics and safety rules, creating a cloned voice requires two human steps: a speaker-permission declaration checkbox in the Create Voice dialog, and a human consent attestation with the speaker's and attester's names. Both are permanently stored in the profile metadata. Automation, headless scripts, and AI agents cannot bypass this human-only gate.

### Prompt-Based Voice Design
Create synthetic voices by entering a natural language description (e.g. *"Warm, deep British documentary narrator with calm cadence"*). The profile records the description and tags; timbre matching is not implemented yet.

### Portable `.alpvoice` Packages
Voice profiles can be exported to and imported from `.alpvoice` packages (standard uncompressed ZIP format with IEEE 802.3 CRC-32 integrity validation). Drag a bundle onto the window or use the per-row **Export** button. Built-in profiles cannot be deleted.

---

## 8. Models & Delivery

- **Models:** The catalogue lists each model with author, version, size, licence
  and on-disk status. Accepting a licence records the decision; SHA-256
  verification is available from `alp-cli`. Audio Labs Pro has no downloader and
  makes no outbound requests, so files are placed in the model folder by hand.
- **Exporting Master Audio:**
  - **Master WAV (24-bit PCM):** Studio broadcast master.
  - **Podcast Preset:** Mastered to -16 LUFS integrated loudness with -1.0 dBFS ceiling.
  - **Audiobook Preset:** -20 LUFS integrated.
  - **Voice-Over Preset:** High-clarity vocal balance optimized for spoken word.
  - **FLAC:** Lossless 24-bit compression.
  - **MP3 and OGG:** Not shipped. MP3 remains rejected for licensing reasons;
    OGG is deferred until the encoder licence is vetted.
