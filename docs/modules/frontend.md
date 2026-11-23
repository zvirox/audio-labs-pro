# `frontend`

Status: planned

## Purpose

The modern web-based graphical user interface for Audio Labs Pro. Implemented with Vite, React, TypeScript (strict mode), and Tailwind CSS, `frontend/` replaces both the iced rendering engine in `alp-ui` and the component library in `alp-ui-kit`.

It delivers:
- The **Dual-Axis Navigation System** (56px vertical primary rail + 40px horizontal contextual subtabs + collapsible 280px inspector drawer).
- The studio component kit (`frontend/src/kit/`) based on accessible Radix UI primitives styled with design tokens ported from `alp-ui-kit`.
- Hardware-accelerated Canvas 2D and WebGL renderers for multi-hour waveform scrolling, clip automation envelopes, and STFT spectrogram heatmaps.
- High-productivity studio workspaces: Voice Lab, Multitrack Arranger, Audiobook Studio, Video Dubbing, Effects Rack, Models, and Settings.
- Keyboard-first interaction via `cmdk` command palette and customizable hotkeys.

## Public API

`frontend/` is an npm/Vite application package rather than a Rust crate. Its top-level structural components and modules include:

- `src/kit/` — Studio design primitives (`Button`, `Slider`, `Toggle`, `Tabs`, `Dialog`, `Tooltip`, `Card`, `Meter`, `Menu`). All application views use the kit exclusively.
- `src/layout/` — `PrimaryRail`, `SubtabTopBar`, `DockedPlayer`, `InspectorDrawer`, and workspace container components.
- `src/views/` — Top-level studio workspaces (`VoiceLab`, `Multitrack`, `AudiobookStudio`, `DubbingStudio`, `Effects`, `Models`, `Settings`).
- `src/canvas/` — High-performance Canvas 2D/WebGL timeline, peak pyramid waveform renderer, and spectral FFT heatmaps.
- `src/ipc/` — Type-safe Tauri command callers, event listeners, and binary channel decoders generated from Rust signatures.
- `src/state/` — Zustand stores managing transient UI view state (zoom level, scroll coordinates, active inspection selection).

## Data flow

1. User clicks or shortcuts trigger actions in React view components.
2. View components dispatch strongly typed IPC commands through `src/ipc/` to `alp-desktop`.
3. The frontend never mutates project data directly; it listens for Rust revision change events and updates the view mirror accordingly.
4. Continuous playhead movement is interpolated locally at 60 fps using `requestAnimationFrame` based on time-stamped transport anchors from Rust.
5. Real-time meter levels (~30 Hz) and binary peak pyramid buffers update Canvas memory without triggering React DOM re-renders.

## Allowed dependencies

- Core dependencies: React, TypeScript, Tailwind CSS, Radix UI primitives, Lucide React, Framer Motion, cmdk, Zustand.
- Offline-only requirement: All fonts (Inter Variable), icon SVGs, and bundles must be packaged locally; zero external network requests or CDN dependencies.
- **Strictly forbidden:** Direct filesystem access or shell execution; all interactions must pass through the typed Tauri IPC bridge.

## Threading / real-time notes

- Executes inside the WebKitGTK JavaScript runtime thread.
- Heavy computational tasks (such as client-side hit testing or large-scale spectrogram color-mapping) are offloaded to Web Workers.
- Waveform and timeline canvas rendering utilize double-buffered Canvas 2D / WebGL contexts to guarantee $\ge 55$ fps rendering during autoscroll.

## Error handling

- React Error Boundaries wrap every workspace domain, preventing view crashes from unmounting the shell.
- IPC command rejections surface through descriptive toast notifications and modal error inspectors.
- Network and model download failures provide retry and resume options.

## Testing

- Unit tests for components, stores, and hooks using Vitest and React Testing Library.
- End-to-end integration tests using Playwright with simulated Tauri IPC responses.
- Canvas hit-testing and timeline mathematics covered by automated test suites.

## Known limits

- DOM nodes are strictly avoided for individual audio clips or samples; rendering massive timelines depends entirely on Canvas 2D / WebGL.
- CSS `backdrop-filter` provides visual blur within the webview frame only, not against external desktop windows.
