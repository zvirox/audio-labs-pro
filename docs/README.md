# Audio Labs Pro — documentation index

Linux-only, English-only, Rust audio studio with local TTS, dictation and voice
profiles. Licensed Apache-2.0.

## Start here

| Document | Purpose |
|---|---|
| [architecture/ARCHITECTURE.md](architecture/ARCHITECTURE.md) | Crates, processes, threading and enforced dependency directions |
| [architecture/STACK.md](architecture/STACK.md) | Chosen crates, versions and licenses |
| [modules/](modules/) | One page per crate and module, with the same required sections |
| [spikes/PHASE0-RESULTS.md](spikes/PHASE0-RESULTS.md) | Phase 0 measurements, evidence and limitations |
| [spikes/PHASE1-RESULTS.md](spikes/PHASE1-RESULTS.md) | Phase 1 acceptance measurements and soak results |
| [spikes/PHASE2-RESULTS.md](spikes/PHASE2-RESULTS.md) | Phase 2 shell, waveform, persistence and glass results |
| [spikes/PHASE3-RESULTS.md](spikes/PHASE3-RESULTS.md) | Phase 3 edit, export and autosave results; hardware gate status |
| [spikes/PHASE4-RESULTS.md](spikes/PHASE4-RESULTS.md) | Phase 4 multitrack results and known limits |
| [spikes/PHASE5-RESULTS.md](spikes/PHASE5-RESULTS.md) | Phase 5 effects, cleanup pipeline and loudness results |
| [spikes/PHASE6-RESULTS.md](spikes/PHASE6-RESULTS.md) | Phase 6 model manager, alp-infer worker and built-in TTS results |
| [spikes/PHASE7-RESULTS.md](spikes/PHASE7-RESULTS.md) | Phase 7 dictation, VAD, subtitle export, and text-based editing results |
| [spikes/PHASE8-RESULTS.md](spikes/PHASE8-RESULTS.md) | Phase 8 voice profiles, cloning safety, voice design, and .alpvoice results |
| [spikes/PHASE9-RESULTS.md](spikes/PHASE9-RESULTS.md) | Phase 9 polish, packaging, first-run wizard, and 1.0 release results |
| [spikes/PHASE10-RESULTS.md](spikes/PHASE10-RESULTS.md) | Phase 10 MCP server, automation layer, session rollback, and AI attach results |
| [spikes/PHASE11-RESULTS.md](spikes/PHASE11-RESULTS.md) | Phase 11 CLAP and LV2 audio plugin hosting engine, CLI, UI, and MCP results |
| [mcp/TOOLS.md](mcp/TOOLS.md) | Model Context Protocol tool catalog, schemas, and permission tiers |
| [mcp/CLIENT_SETUP.md](mcp/CLIENT_SETUP.md) | Client connection guides for Claude Desktop, Cursor, and VS Code |
| [mcp/AGENT_GUIDE.md](mcp/AGENT_GUIDE.md) | Operational guidelines and principles for LLM agents interacting with ALP |
| [mcp/WORKFLOWS.md](mcp/WORKFLOWS.md) | Multi-step agent workflow recipes (cleaning, podcast, audiobook, narration) |
| [mcp/SECURITY.md](mcp/SECURITY.md) | Security model, sandboxing, audit trails, and human-only consent invariants |
| [USER_GUIDE.md](USER_GUIDE.md) | Comprehensive workstation manual for recording, editing, voice, and export |
| [KEYBOARD_SHORTCUTS.md](KEYBOARD_SHORTCUTS.md) | Full hotkey cheat sheet |
| [ETHICS.md](ETHICS.md) | Voice cloning ethics, human consent gate, and acceptable-use policies |
| [TROUBLESHOOTING.md](TROUBLESHOOTING.md) | Platform setup guide for Wayland, NVIDIA, PipeWire, and ALSA |
| [adr/](adr/) | Architecture Decision Records |
| [../THIRD_PARTY.md](../THIRD_PARTY.md) | Every crate, model and asset with its license |
| [../CHANGELOG.md](../CHANGELOG.md) | Release history |

## Phase status

| Phase | State |
|---|---|
| 0 — research spikes and decisions | complete; evidence in `spikes/` |
| 1 — foundation | complete: workspace, core model, media, engine, CLI, guards, CI |
| 2 — UI shell and glass system | complete on this machine; KDE/GNOME matrix limitations documented |
| 3 — recording and single-track editing | complete on this machine: edits, capture, export and shell controls; hardware soak and forced-kill recovery pass |
| 4 — multitrack timeline and mixing | complete: engine pan, master bus, mixer UI, armed imports, crossfade options, delivery presets (podcast/audiobook/voice-over), markers/loop range, and stems pass |
| 5 — effects and voice cleanup (v0.1) | complete: biquads, 5-band voice EQ, compressor, noise gate, peak limiter, de-esser, de-hum, de-click, ITU-R BS.1770 LUFS metering/normalization, one-click voice cleanup preset, CLI tools, and UI effects panel integration |
| 6 — model manager and built-in TTS | complete: ModelManager, SHA-256 verification, licensing, out-of-process alp-infer daemon over Unix socket, text normalizer, pronunciation dictionary, script mode (dirty line regen & stems), CLI tools, and UI Voice Lab integration |
| 7 — dictation and transcription | complete: dual-threshold VAD speech segmentation, streaming DictationSession with sub-second feedback, Transcript document model with TXT/SRT/VTT export & parsing, text-based ripple cutting, CLI commands, and UI Voice & Dictation Lab integration |
| 8 — voice profiles, cloning, and voice generation | complete: ProfileStore, prebuilt voices ("Aura", "Atlas", "Echo", "Sol"), .alpvoice ZIP packaging, enforced human consent gate for clones, reference quality audit, voice design from prompt, generation cache, A/B comparison, and hardware tiering |
| 9 — polish, packaging, and 1.0 | complete: Decision D12 (io.github.zvirox.AudioLabsPro), AppImage/Flatpak/Debian packaging, desktop/MIME integration, fuzzing/stress benchmarks, first-run wizard, keyboard shortcuts modal, user guide, and 1.0 release |
| 10 — MCP: AI-driven editing, creation, and export | complete: Release v1.1 "Agent", alp-api automation layer, alp-mcp server (spec revision 2026-07-28), 14 core tools, 8 prompt workflows, sandboxing, session rollback, and UI live attach control panel |
| 11 — CLAP & LV2 Audio Plugin Hosting | complete: Release v1.2 "Plugins", alp-plugin engine, C ABI loaders for CLAP and LV2, real-time safety, scanner/cache, CLI scan/list/inspect, MCP tools, and the hosted-plugin browser in the Effects workspace |
| Unreleased — seven-workspace retro shell | in progress: Generate, Stories, Captures, Voices, Effects, Models and Settings workspaces on the retro palette; see `modules/alp-ui/app/` |

## Phase 11 acceptance

Reproduce the full CLAP & LV2 plugin hosting suite with `bash scripts/phase11-demo.sh`:

| Check | Result |
|---|---|
| Architecture & Safety Guards | `check-file-length.sh`, `check-deps.sh`, `check-docs.sh`, `check-ui.sh` pass cleanly |
| Open Standards | Dynamic C ABI loading for CLAP 1.2+ (`clap.h`) and LV2 (`lv2.h`) |
| Real-Time Safety | Preallocated stereo scratch buffers, zero heap allocations in audio loop |
| Discovery & Cache | Standard paths (`/usr/lib/clap`, `~/.clap`, `/usr/lib/lv2`, `~/.lv2`), cached in `$XDG_CACHE_HOME/alp/plugins.json` |
| CLI Commands | `alp-cli plugin scan`, `plugin list`, `plugin inspect <id>` |
| MCP AI Tools | `plugin_list`, `plugin_insert`, `plugin_param_set` |
| Desktop UI | Effects workspace "Hosted Plugins" card with per-row insertion and rescan |

## Phase 10 acceptance

Reproduce the headless MCP tool automation suite with `bash scripts/phase10-demo.sh`:

| Check | Result |
|---|---|
| Workspace validation guards (`fmt`, `clippy -D warnings`, file-length, deps, docs, ui) | green |
| Stdio JSON-RPC 2.0 protocol (`2026-07-28`) | handshake, discovery, tool listing, execution verified |
| AI Tool suite | `project_info`, `track_add`, `track_list`, `analyze_loudness`, `checkpoint_create`, `export_audio`, `rollback_session` verified |
| Sandboxing & Safety | Directory jail with path traversal (`..` and symlink escape) prevention |
| Session Rollback | Complete rollback to session start checkpoint restores initial zero-edit state |
| Persistent Audit Trail | Structured JSONL logging with timestamps, client IDs, and revision deltas |

## Phase 3 acceptance

Reproduce the headless part with `bash scripts/phase3-demo.sh`:

| Check | Result |
|---|---|
| `cargo fmt`, `clippy -D warnings`, 144 tests | green |
| Dependency, file-length, module-doc and UI-widget guards | green |
| Import conversion | 44.1 kHz source converted to 48 kHz |
| Autosave recovery | clean project 3 clips; recovered edited project 4 clips |
| WAV/FLAC null test | max difference 1.8e-7 at 24-bit; threshold 0.001 |
| 30-minute dropout soak | 86,401,536 frames · 0 dropped samples |
| Forced-kill capture recovery | 379,904 frames · `recovered=true` |

## Phase 2 acceptance

Reproduce with `cargo test --workspace` and the diagnostic command in
`docs/spikes/PHASE2-RESULTS.md`:

| Check | Result |
|---|---|
| `cargo fmt`, `clippy -D warnings`, 104 tests | green |
| Dependency, file-length, module-doc and UI-widget guards | green |
| 1-hour waveform autoscroll | 60–61 frames/s, 16.6 ms average frame time |
| Idle CPU with nothing animating | 0.00% over a 7 s sample (0 scheduler ticks) |
| Idle RSS | about 86 MB |
| Inter | bundled variable font, family `Inter Variable`, OFL text shipped |
| Panels persist | versioned `workspace.json` + atomic write, unit-tested round-trip |
| Workers do not block UI | cancellable std-thread jobs + non-blocking channel, tested |
| Raw iced buttons in `alp-ui` | none; `scripts/check-ui.sh` enforces |
| Glass | KWin/X11 property path; tint fallback on Wayland/GNOME |

## Conventions

- Code lives in `crates/`; Phase 0 evidence lives in `spikes/` and is never
  linked into the shipping workspace.
- Every module gets a document under `modules/` in the same commit as the code.
  The required sections are Purpose, Public API, Data flow, Allowed
  dependencies, Threading / real-time notes, Error handling, Testing and Known
  limits.
- Decisions that change the architecture are recorded in `adr/` before the code
  that depends on them.
