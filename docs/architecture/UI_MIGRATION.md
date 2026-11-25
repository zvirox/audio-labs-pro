# UI Architecture & Migration Record (Complete & Locked)

Audio Labs Pro completed the full migration from `iced` (`alp-ui` / `alp-ui-kit`) to a Tauri v2 + React 19 desktop architecture and finalized its permanent **Linear Design System**.

---

## Migration Summary (M0–M8 Complete)

| Milestone | Scope | Status |
|---|---|---|
| M0 | Canvas waveform scroll/zoom latency & IPC benchmarks | Done |
| M1 | Extracted framework-agnostic `alp-session` core | Done |
| M2 | Scaffolded `alp-desktop` (Tauri v2) & `frontend/` (Vite + React 19 + TypeScript) | Done |
| M3 | Implemented UI primitives (`frontend/src/kit/`), dual-axis shell, shortcuts, command palette | Done |
| M4 | Real-time transport ticker, discrete segmented meters, zoomable `WaveformCanvas` | Done |
| M5 | Ported all 9 workspaces (`Dashboard`, `Voices`, `Generate`, `Multitrack`, `Stories`, `Captures`, `Effects`, `Models`, `Settings`) | Done |
| M6 | Interactive HTML5 Canvas 2D `MultitrackCanvas` (snapping, ripple edit, envelopes, split/trim/slip) | Done |
| M7 | Full MCP & AI attach UI parity (`SettingsWorkspace`, consent/approval banners, live activity feed) | Done |
| M8 | Deleted `alp-ui` / `alp-ui-kit` crates; unified `.deb`/`.rpm`/`.pkg.tar.zst` packaging via `packaging/nfpm.yaml` | Done |
| Post-M8 | Spectral Multi-View (Phase 13) + **Final Locked Linear UI System** | Done |

---

## Locked Linear UI Standard (Permanent Specification)

Any future component added under `frontend/src/` MUST obey these locked rules (enforced automatically by `npm --prefix frontend run check:linear`):

1. **Window Geometry & Surface:** `1280x800` default (16:10), `1024x640` minimum, `decorations: false`, `transparent: false`, `shadow: true`, zero compositor/KWin/X11 blur.
2. **Universal `TitleBar` (40 px):** Contains logo (20 px), app title, version tag read from `tauri.conf.json`, clean drag region, the **sole** application `ThemeToggle` (`ui-theme` storage key), divider, minimize, maximize/restore, and close (`#E5484D` hover).
3. **Tokens & Solid Surfaces:**
   - Dark (`data-theme="dark"`): `--bg-app: #050505`, `--bg-surface: #111213`, `--bg-surface-2: #1A1B1E`, `--border: #1F2125`, `--border-strong: #2C2E33`, `--text: #F5F5F6`.
   - Light (`data-theme="light"`): `--bg-app: #F3F4F6`, `--bg-surface: #FFFFFF`, `--bg-surface-2: #F4F5F7`, `--border: #E5E7EB`, `--border-strong: #D1D5DB`, `--text: #0F1115`.
   - Primary accent `#3355FF`, sky `#38B6F0`, green `#22C55E`, red `#F43F5E`, amber `#EAB308`, teal `#1FB5A5`, purple `#7C5CE6`, orange `#F97316`.
4. **Forbidden Patterns:** Zero `backdrop-filter`/blur, zero translucent surface overlays (`rgba()`, `hsla()`, Tailwind `/10` `/20`), zero `shadowBlur` or glow/pulse animations, zero surface gradients (`bg-gradient-*`), zero drop shadows in dark mode, zero pill-shaped chips/buttons (`rounded-full` allowed strictly on tiny status/legend dots).
5. **Typography:** Self-hosted `InterVariable.ttf`; `tabular-nums` for stat cards and tables; monospace restricted solely to timecodes, keycaps, and terminal/log strings.
6. **Canvases:** `WaveformCanvas`, `SpectrogramCanvas`, and `MultitrackCanvas` read live theme variables via `linearCanvasTheme.ts` on every `theme-changed` event so light and dark themes both stay crisp and high-contrast.
