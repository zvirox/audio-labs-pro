# ADR 0021: Phase 11 — CLAP and LV2 Audio Plugin Hosting Architecture

- **Status:** Accepted
- **Date:** 2026-10-27
- **Deciders:** Antigravity Engineering Team
- **Consulted:** Linux Audio Developers, Bitwig/u-he CLAP Working Group, LV2 Specification Community

## Context and Problem Statement

Audio Labs Pro provides a comprehensive suite of built-in DSP effects (5-band parametric voice EQ, noise gate, optical-style compressor, peak limiter, transient de-clicker, multi-harmonic de-hum, sibilance detector, and ITU-R BS.1770 LUFS normalizer). However, professional podcast producers, voice actors, and mastering engineers frequently rely on specialized external third-party audio plugins for creative processing, analog emulation, noise reduction (e.g. RNNoise, iZotope, FabFilter), and acoustic correction.

On Linux, the two modern open audio plugin formats are:
1. **CLAP (CLever Audio Plugin):** An open, non-proprietary C ABI specification with comprehensive thread safety, explicit audio bus negotiation, parameter automation, and state management.
2. **LV2:** The established Linux audio plugin ecosystem with wide distribution across Linux repositories (`/usr/lib/lv2`), including suites like Calf Studio Plugins, LSP (Linux Studio Plugins), and RNNoise.

We need a dedicated, safe, modular, and real-time safe plugin hosting layer (`alp-plugin`) that discovers, instantiates, automates, and renders CLAP and LV2 plugins inside Audio Labs Pro's multitrack timeline and export pipeline.

## Decision Drivers

1. **Safety and Isolation:** Dynamic library loading must be robust against faulty plugins and never compromise host memory stability.
2. **Hard Real-Time Performance:** Plugin audio callbacks (`process` / `run`) must not allocate heap memory, acquire blocking mutexes, or perform system calls during playback.
3. **Open Standards:** Strictly non-proprietary formats (CLAP and LV2). VST2 is legally restricted; VST3 carries complex C++ COM interfaces with licensing overhead. CLAP and LV2 provide clean C ABIs.
4. **Standard Linux Discovery Paths:** Conform to the Linux Audio standard directory hierarchy (`/usr/lib/clap`, `~/.clap`, `/usr/lib/lv2`, `~/.lv2`, and `CLAP_PATH`/`LV2_PATH` environment variables).
5. **State Serialization:** Project files (`.alp`) must serialize and restore plugin state losslessly using compact base64 data payloads.
6. **Automation & MCP Exposure:** External plugins must be inspectable and automatable both interactively in the GUI and headlessly via the MCP server.

## Considered Options

- **Option 1: In-process C ABI Loader with `libloading` (Chosen)**
  - Load plugins directly into the audio engine process using standard dynamic loading.
  - Pre-allocate all audio scratch buffers, parameter queues, and descriptors.
  - Native performance with zero inter-process IPC latency or context-switch overhead.
- **Option 2: Out-of-Process Plugin Host Sidecar (Crash Isolation)**
  - Run plugins in a separate daemon via shared memory ringbuffers.
  - Advantage: Plugin crashes do not crash the host app.
  - Disadvantage: Introduces scheduling jitter, latency, and IPC complexity unsuitable for low-latency voice monitoring.
- **Option 3: External Wrapper Tools (e.g. Carla)**
  - Defer plugin hosting to external tools like Carla or yabridge.
  - Disadvantage: Adds heavy external runtime dependencies, breaks self-contained packaging, and hinders headless automation.

## Decision Outcome

Chosen: **Option 1 (In-process C ABI Loader with `libloading`)**, coupled with strict validation, preallocated audio buffers, and exception guards.

### Architecture and Component Layout

```
                  ┌──────────────────────────────────────────────┐
                  │                 alp-ui / CLI                 │
                  └──────────────┬───────────────────────────────┘
                                 │
                                 ▼
                  ┌──────────────────────────────────────────────┐
                  │                   alp-api                    │
                  │   (Plugin Discovery, Insert, Automate Tools) │
                  └──────────────┬───────────────────────────────┘
                                 │
                                 ▼
                  ┌──────────────────────────────────────────────┐
                  │                  alp-plugin                  │
                  │  ┌──────────────┐          ┌──────────────┐  │
                  │  │ CLAP Host    │          │  LV2 Host    │  │
                  │  │ Engine       │          │  Engine      │  │
                  │  └──────┬───────┘          └──────┬───────┘  │
                  │         │                         │          │
                  │         ▼                         ▼          │
                  │  ┌────────────────────────────────────────┐  │
                  │  │    ActivePlugin Instance Interface     │  │
                  │  └────────────────────────────────────────┘  │
                  └──────────────┬───────────────────────────────┘
                                 │
                                 ▼
                  ┌──────────────────────────────────────────────┐
                  │                  alp-engine                  │
                  │ (Inline Track Effect Processing & Mix Bus)   │
                  └──────────────────────────────────────────────┘
```

### Key Technical Specifications

1. **Crate `alp-plugin`:**
   - Pure-Rust hosting layer depending on `alp-core`, `libloading`, `serde`, `serde_json`, and `thiserror`.
   - `Scanner`: Discovers plugins in standard Linux paths and caches metadata in `$XDG_CACHE_HOME/alp/plugins.json`.
   - `ClapHost`: Implements `clap_host` callbacks, handles `clap_plugin_entry` and `clap_plugin_factory`, coordinates `activate`, `process`, and `deactivate`.
   - `Lv2Host`: Reads `manifest.ttl`, dynamically links plugin shared objects, creates `LV2_Handle` instances, wires audio ports, and executes `run()`.
   - `ActivePlugin`: Thread-safe, unified abstraction for parameter lookup, automation, real-time block rendering, and state snapshotting.

2. **Project Persistence (`alp-core`):**
   - New `EffectType::HostedPlugin`:
     ```rust
     HostedPlugin {
         format: PluginFormat, // Clap or Lv2
         plugin_id: String,
         plugin_name: String,
         state_base64: String,
         parameters: HashMap<String, f32>,
     }
     ```

3. **Real-Time Processing Invariant:**
   - Input and output audio buffers are preallocated to the maximum engine buffer size (default 4096 frames).
   - Zero memory allocations, lock acquisitions, or filesystem I/O are performed inside the audio callback loop.

## Consequences

### Positive
- Users can load and automate industry-standard audio plugins (RNNoise, Calf, LSP, u-he, FabFilter, etc.) directly in Audio Labs Pro.
- Preserves self-contained desktop packaging without requiring external jack/pipewire bridges.
- Full MCP and headless CLI integration enables AI agents to select, insert, and adjust third-party mastering plugins.

### Negative / Risks
- In-process loading means an unhandled segmentation fault inside a third-party C/C++ plugin could crash the application.
- *Mitigation:* Audio Labs Pro's autosave engine (tested in Phase 3/9) ensures zero project data loss even in the event of an abrupt process kill. Plugins are validated before instantiation.
