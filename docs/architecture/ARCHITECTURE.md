# System Architecture

Audio Labs Pro is a local-first Linux workstation combining non-destructive audio production, real-time DSP, neural speech synthesis/transcription, plugin hosting, and authenticated AI MCP control inside a single-window Tauri desktop application.

---

## Crate Topology (12 Crates, Acyclic DAG)

```
alp-core                  Domain model: Project, Track, Clip, Fade, Envelope, BlockStore, Undo/Redo
  ├── alp-media           Audio decoding (WAV/FLAC/OGG/MP3), peak pyramids, resampler, TPDF export
  ├── alp-dsp             Parametric EQ, biquads, compressor/limiter, LUFS, RNNoise/spectral denoise, STFT spectrogram
  └── alp-plugin          CLAP & LV2 plugin discovery, sandboxing, and parameter state
        │
alp-engine                cpal/PipeWire/ALSA live audio loop, lock-free mixer, capture, offline renderer
alp-voice                 Voice cloning consent & curation, lexicon, VAD, scripts, STT & synthesis orchestration
alp-infer                 Isolated inference subprocess daemon (Kokoro, Kitten, StyleTTS2, Qwen3, Moonshine, GGUF)
        │
alp-api                   High-level workspace/track/clip/voice/effect/job/audit/export API operations
alp-session               Framework-agnostic UI session bridge: workspace state, panels, transport ticker, undo/AI wiring
        │
  ├── alp-cli             Headless CLI automation (`alp`)
  ├── alp-mcp             JSON-RPC stdio/UNIX socket server (68 tools across Observe/Edit/Export) + Starship bridge
  └── alp-desktop         Tauri v2 shell (`audio-labs-pro`), frameless opaque Linear window, IPC commands & events
        └── frontend/     React 19 + TypeScript + Vite 8 + Tailwind CSS v4 + ECharts + cmdk Linear UI
```

---

## Key Architectural Decisions

1. **Zero Audio-Thread Blocking:** All file decoding, neural inference, SQLite block commits, and IPC state serialization stay off the real-time audio callback thread.
2. **Process-Isolated Neural Inference:** Heavy ML runtimes (`onnxruntime` / `llama.cpp` / Candle) execute inside the `alp-infer` child binary so model crashes cannot crash or glitch live audio playback.
3. **Consent-Locked Voice Cloning:** Cloning flows in `alp-voice` and `CloningConsentModal` require explicit speaker name, attesting identity, and legal attestation before generating embeddings.
4. **Framework-Agnostic Session Kernel (`alp-session`):** All application state, panel layout clamping, transport synchronization, and AI attach approval logic live in `alp-session` so `alp-desktop` (Tauri) and `alp-mcp` operate over identical invariants.
5. **Final Locked Linear UI (`frontend/` + `alp-desktop`):**
   - Frameless desktop window (`1280x800` default, `1024x640` minimum, `decorations: false`, `transparent: false`, `shadow: true`) with opaque surfaces and zero compositor/X11 blur (`crates/alp-desktop/src/glass.rs` enforces opaque fallback).
   - Strict token hierarchy (`frontend/src/tokens.css`, `index.css`) supporting dark (`#050505` base / `#111213` surface) and light (`#F3F4F6` base / `#FFFFFF` surface) themes persisted under `ui-theme`, switched exclusively from `TitleBar` (`Ctrl+D`), with self-hosted `InterVariable.ttf` and zero glass/blur/glow/translucency enforced by `npm run check:linear`.
6. **Starship Control Bridge:** `alp-mcp::starship` integrates `starship-plugin` to expose Audio Labs Pro health and session status over `$XDG_RUNTIME_DIR/starship/bridges/alp.sock`.

---

## Verification Guards

Every repository change must pass automated guards in `scripts/` and `frontend/scripts/`:
- `./scripts/check-deps.sh` — Crate DAG boundaries
- `./scripts/check-docs.sh` — 8-section module docs per Rust file
- `./scripts/check-file-length.sh` — Max 1,500 lines per Rust source file
- `./scripts/check-ui.sh` — `frontend/src/kit/` primitives and accessibility checks
- `npm --prefix frontend run check:linear` — Enforces locked Linear UI design constraints
