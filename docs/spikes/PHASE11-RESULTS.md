# Phase 11 Results — CLAP & LV2 Audio Plugin Hosting

- Date: 2026-10-27
- Deliverables: `alp-plugin` hosting engine crate, dynamic C ABI loaders for CLAP and LV2 standards, discovery scanning, parameter automation, real-time audio thread processing, CLI tool suite, UI effects panel integration, and MCP automation.

---

## 1. Architectural Decisions Resolved

- **ADR 0021 (CLAP & LV2 Hosting Architecture):**
  - Standardized on modern, open Linux audio plugin formats: CLAP 1.2+ (`clap.h`) and LV2 (`lv2.h`).
  - Prohibited proprietary or unmaintained legacy formats (VST2, LADSPA).
  - Maintained safe, isolated Rust abstractions over dynamic `libloading` boundaries with zero unsafe exposure to callers.
  - Implemented preallocated, allocation-free real-time block processing (`TrackPluginChain`) ensuring no heap allocations or lock contention inside audio callback threads.

---

## 2. Capabilities Implemented

- **`alp-plugin` Crate:**
  - `clap`: CLAP C ABI struct representations, host negotiation, factory discovery, parameter querying, and stereo block execution.
  - `lv2`: LV2 C ABI struct definitions, `manifest.ttl` parsing, port connection routing, and activation lifecycle.
  - `instance`: Unified `ActivePlugin` wrapping CLAP, LV2, and built-in virtual fallbacks with normalized parameter clamping and JSON state serialization.
  - `scanner`: Multi-path filesystem scanner searching standard Linux locations (`/usr/lib/clap`, `~/.clap`, `/usr/lib/lv2`, `~/.lv2`, and `CLAP_PATH`/`LV2_PATH`), caching metadata in `$XDG_CACHE_HOME/alp/plugins.json`.
  - `host`: `PluginHost` managing discovery and instantiation, and `TrackPluginChain` executing series processing over interleaved and split stereo buffers.
- **`alp-engine` Integration:**
  - Integrated `TrackPluginChain` into `Mixer`, allowing real-time plugin processing on master and track buses with zero callback allocations.
- **`alp-core` Project Serialization:**
  - Enhanced `EffectRef` with optional `state` payload to persist hosted plugin settings across `.alp` project files.
- **`alp-cli` Commands:**
  - `alp-cli plugin scan`: Scan standard and custom paths and populate metadata cache.
  - `alp-cli plugin list`: Display discovered CLAP and LV2 plugins with formats, categories, and paths (text and JSON).
  - `alp-cli plugin inspect <id>`: Inspect plugin metadata and exposed automation parameters with defaults and ranges.
- **`alp-api` & `alp-mcp` Tools:**
  - `plugin_list`: Enumerate discovered audio plugins.
  - `plugin_insert`: Insert a CLAP or LV2 plugin onto a track's effect chain.
  - `plugin_param_set`: Automate parameter values on inserted plugins.
- **`alp-ui` Effects Panel:**
  - Added "Hosted Plugins (CLAP & LV2)" section in the Voice Editing workflow tab.
  - One-click plugin insertion and instant rescan controls.

---

## 3. Real System Verification

- **Installed System Plugins Discovered:**
  - `lsp-plugins` (`/usr/lib/lv2/lsp-plugins.lv2/lsp-plugins-lv2.so`)
  - `rnnoise mono` (`/usr/lib/lv2/rnnoise_mono.lv2/librnnoise_mono.so`)
  - `rnnoise stereo` (`/usr/lib/lv2/rnnoise_stereo.lv2/librnnoise_stereo.so`)
  - `calf` (`/usr/lib/lv2/calf.lv2/calf.so`)
- **Virtual Built-in Plugins:**
  - `ALP Vintage Tube Saturation` (`alp.vintage_tube`)
  - `ALP Master Multiband Dynamics` (`alp.master_multiband`)
  - `ALP Neural Voice Denoiser` (`alp.neural_denoiser`)

---

## 4. Acceptance Harness

The complete phase is verified via [`scripts/phase11-demo.sh`](file:///home/zvirox/Projects/audio-labs-pro/scripts/phase11-demo.sh), validating:
1. Workspace architecture guard checks (`check-file-length.sh`, `check-deps.sh`, `check-docs.sh`, `check-ui.sh`).
2. Release binary compilation for `alp-cli`, `alp-mcp`, and `alp-ui`.
3. CLI plugin scanning, listing, and inspection.
4. Host engine processing and parameter modulation.
5. MCP stdio JSON-RPC automation of `plugin_list`, `track_add`, `plugin_insert`, and `plugin_param_set`.
