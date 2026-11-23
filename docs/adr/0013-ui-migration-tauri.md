# 0013 — UI migration from iced to Tauri 2 and React

- **Status:** Proposed - pending M0 spike
- **Date:** 2026-11-23
- **Decider:** Antigravity Engineering Team
- **Supersedes:** Supersedes the UI parts of ADR 0003 and ADR 0012 (once accepted)
- **Reference Plan:** [`docs/architecture/UI_MIGRATION.md`](../architecture/UI_MIGRATION.md)

## Context

Audio Labs Pro's desktop shell and component kit were initially built using iced 0.14 (`alp-ui`, `alp-ui-kit`), winit, and wgpu. This foundation established early feasibility, proof of non-destructive single-track editing, and compositor integration (ADR 0003 and ADR 0012).

However, as the application expands into a comprehensive voice studio and multitrack DAW arrangement (incorporating the dual-axis navigation shell, interactive multitrack canvas, 2D spectral selection, rich voice persona cards, and script editing), iced presents significant friction:
1. **Visual Polish & Modern Controls:** Reaching commercial studio polish (Voicebox / VoiceStudio level) requires advanced layout ergonomics, dynamic menus, floating palettes, rich SVG/canvas data visualizations, and robust accessibility primitives that require hand-rolling custom widget architectures in iced.
2. **Ecosystem & Velocity:** Web frontend technologies offer mature, accessible component ecosystems (Radix primitives, Tailwind CSS design tokens, Lucide, Framer Motion, cmdk), rich font typography, and high-performance Canvas 2D/WebGL rendering.
3. **Architecture Decoupling:** The current `alp-ui` crate intermingles headless session state (`EditSession`, project command stack, worker queues, AI control socket hosting, autosave management) with UI rendering logic.

The core domains—`alp-engine` (real-time audio), `alp-core` (project model and commands), `alp-dsp` (effects and spectral processing), `alp-media` (audio I/O and peak pyramids), `alp-voice` (profiles and synthesis protocols), `alp-infer` (isolated ONNX/Candle worker), `alp-plugin` (CLAP/LV2 host), and `alp-mcp` (agent tools)—are strictly headless, real-time safe, and high-performance. They do not depend on the GUI toolkit.

## Decision

Migrate the desktop UI presentation layer from iced 0.14 to **Tauri 2** with a **React + TypeScript (strict) + Tailwind CSS** frontend, executed across phased milestones M0 through M8.

### Key Architectural Tenets:

1. **Decouple Session State (`alp-session`):**
   - Extract all non-rendering session logic out of `alp-ui` into a new headless crate `alp-session`.
   - `alp-session` owns `EditSession`, the undoable `Command` stack, the background worker queue, the native `alp-api::control` socket host, autosave persistence, and project state synchronization.
   - `alp-session` depends on domain crates (`alp-core`, `alp-engine`, `alp-media`, `alp-voice`, `alp-api`) but **must never depend on iced or tauri**.

2. **Tauri 2 Native Desktop Shell (`alp-desktop`):**
   - Create `alp-desktop` as the native desktop entrypoint using Tauri 2.
   - Provides native window configuration, system menus, IPC dispatch (commands, events, batched channels), and the Linux glass/blur module (`x11rb`).
   - `alp-desktop` depends on `alp-session` and the Tauri runtime.

3. **Webview Presentation Layer (`frontend/`):**
   - Single-page application built with Vite, React 19 / 18, TypeScript (strict mode), and Tailwind CSS.
   - Component system in `frontend/src/kit/` using Radix UI primitives with design tokens ported from `alp-ui-kit`.
   - View state managed via Zustand; project state of record remains exclusively in Rust (`alp-session`).
   - Multitrack timeline and waveform rendering use Canvas 2D (or WebGL if required by M0 benchmarks), consuming binary peak pyramids via raw IPC buffers.
   - Command palette powered by `cmdk`.

4. **Strict IPC Boundary:**
   - TS $\to$ Rust: `invoke` commands for user actions (`DeleteRange`, `PasteClips`, transport, export, import).
   - Rust $\to$ TS: typed Events for revision updates, job progress, model download notifications, and AI attach status.
   - Rust $\to$ TS: batched `Channel` streaming for high-rate metering (~30 Hz) and dictation partials.
   - Playhead position interpolated in the frontend via `requestAnimationFrame` anchored on transport state changes rather than per-frame IPC.
   - Types generated deterministically using a Rust-to-TypeScript type generator (`tauri-specta` or `ts-rs`).

5. **Superseding Prior ADRs (Rendering Layer Only):**
   - Once accepted upon completion of the M0 feasibility spike, this decision supersedes the UI-specific parts of **ADR 0003** (window blur and iced winit handle exposure) and **ADR 0012** (Phase 2 iced shell architecture and widget constraints) strictly for the rendering layer.
   - All other glass invariants from ADR 0012 are preserved unchanged:
     - The session/desktop matrix remains identical.
     - KWin/X11 blur through `_KDE_NET_WM_BLUR_BEHIND_REGION` via safe `x11rb`.
     - Translucent tint fallback (alpha $\ge$ 0.70) on Wayland (GNOME, niri, KDE/Wayland).
     - `ALP_FORCE_TINT=1` overrides compositor blur for testing and accessibility.
     - Wayland `ext_background_effect_v1` remains out of scope due to unsafe foreign display requirements.
     - Motion is decorative and never encodes audio state.
   - The glass module moves from `alp-ui::glass` to `alp-desktop::glass`.
   - CSS `backdrop-filter` is documented as an inner-panel effect only, since WebKitGTK blurs webview DOM content rather than desktop windows.

## Considered Alternatives

### 1. Stay on iced 0.14 (Status Quo)
- **Pros:** 100% pure Rust stack; lightweight static executable; zero webview dependencies.
- **Cons:** High cost of authoring custom UI controls, lack of production-ready accessible component libraries, high maintenance for complex docking/tab systems and multi-track canvas editing; slow design iteration velocity compared to modern CSS/component frameworks.
- **Outcome:** Rejected due to long-term ergonomic and design polish ceilings.

### 2. Electron
- **Pros:** Mature desktop web wrapper; access to full Chromium and Node.js ecosystems; extensive cross-platform tooling.
- **Cons:** Large binary size (+150MB overhead), high idle memory footprint (typically 150–250MB RSS vs Tauri's 40–80MB), bundled Node runtime security attack surface, and slower IPC serialization compared to Tauri's direct Rust native bindings.
- **Outcome:** Rejected. Tauri 2 delivers webview UI fidelity while retaining minimal memory footprint, fast startup, and native Rust IPC performance.

## Consequences

### Positive
- Rapid UI feature velocity using modern React component libraries and Tailwind design tokens.
- Complete separation between view presentation and audio/project session state.
- Clean headless testing of `alp-session` independent of any graphical display server.
- Significantly richer visual feedback (radar charts, interactive spectrogram heatmaps, drag-and-drop arranger, floating inspect drawers).

### Neutral / Trade-offs
- **Linux Distro Dependencies:** WebKitGTK (`webkit2gtk-4.1` / `javascriptcoregtk-4.1`) becomes a required shared system library on target distributions.
- **Packaging:** Flatpak runtime moves at Phase M8 to a runtime that ships WebKitGTK (GNOME runtime family). Runtime version: verify the currently supported GNOME runtime and that it provides `webkit2gtk-4.1` at M8 time (do not hardcode a specific GNOME runtime version ahead of time). An early de-risk spike in Phase M2 will build a throwaway Flatpak manifest with the Tauri skeleton to confirm audio (PipeWire/ALSA), file access, and model-directory access with exit criterion "manifest builds and plays a test tone". The current Freedesktop 24.08 runtime remains active for the iced application until M8.
- **Compiler Invariants (`unsafe_code = "deny"`):** Keep the workspace-wide deny invariant intact. In Phase M2, verify whether Tauri 2's generated code actually trips `#![deny(unsafe_code)]`. If it does not trip, no exception exists. If it does trip, any exception is strictly scoped to `alp-desktop` only and applied at the narrowest possible level (the specific macro-generated item, never crate-wide). Hand-written code in `alp-desktop` must contain zero `unsafe` (glass and window code use safe APIs only: `x11rb` and `raw-window-handle`). Every required exception must be cataloged in ADR 0013 with the reason, exact item, and Tauri version, and re-audited on each Tauri upgrade. A planned guard (`check-deps.sh` or a dedicated check script) will enforce that `unsafe` never appears in hand-written `alp-desktop` source.
- **Dual Maintenance:** The existing iced application remains the stable shipping product throughout phases M0–M7. `alp-ui` and `alp-ui-kit` are removed only at Phase M8 after achieving full functional parity.
