# UI Architecture & Migration Record (Complete & Locked)

Audio Labs Pro completed the full migration from `iced` (`alp-ui` / `alp-ui-kit`) to a Tauri v2 + React 19 desktop architecture and finalized its permanent **Linear Design System** using the shared family palette with a **Magenta** primary accent (`DESIGN.md`).

---

## Migration Summary (M0–M8 Complete)

| Milestone | Scope | Status |
|---|---|---|
| M0–M7 | Telemetry benchmarks, `alp-session`, Tauri v2 shell, UI kit, workspaces, Multitrack canvas, AI/MCP parity | Done |
| M8 | Deleted `alp-ui` / `alp-ui-kit` crates; unified `.deb`/`.rpm`/`.pkg.tar.zst` packaging via `packaging/nfpm.yaml` | Done |
| Post-M8 | Spectral Multi-View (Phase 13), **Shared Palette + Magenta Primary Lock**, and Chart Removal (`echarts` dropped) | Done |

---

## Locked Linear UI Standard (Permanent Specification)

Any future component added under `frontend/src/` MUST obey these locked rules (enforced automatically by `npm --prefix frontend run check:linear`):

1. **Window Geometry & Surface:** `1280x800` default (16:10), `1024x640` minimum, `decorations: false`, `transparent: false`, `shadow: true`, zero compositor/KWin/X11 blur.
2. **Universal `TitleBar` (40 px):** Logo (20 px), app title, version tag read from `tauri.conf.json`, clean drag region, the **sole** application `ThemeToggle` (`ui-theme` storage key), divider, minimize, maximize/restore, and `#E5484D` (`--close-hover`) hover close.
3. **Shared Tokens (`src/styles/linear-theme.css` + `src/styles/app-accent.css`):**
   - Dark (`data-theme="dark"`): `--bg-app: #1C1C1C`, `--bg-sidebar: #1C1C1C`, `--bg-surface: #1C1C1C`, `--bg-surface-2: #292929`, `--bg-active: #292929`, `--bg-hover: #232323`, `--border: #2E2E2E`, `--border-strong: #363636`, `--text: #FFFFFF`, `--text-secondary: #CBCBCB`.
   - Light (`data-theme="light"`): `--bg-app: #F8F9FA`, `--bg-sidebar: #F3F4F6`, `--bg-surface: #FFFFFF`, `--bg-surface-2: #F3F4F6`, `--bg-active: #E5E7EB`, `--border: #E5E7EB`, `--border-strong: #D1D5DB`, `--text: #111827`.
   - Primary Accent (Magenta): `--primary: #C026D3` / `#A21CAF`, `--primary-text: #E879F9` / `#A21CAF`, `--primary-tint: #331D36` / `#F8EFF9`.
   - Status & Audio Series: `--sky` (`#2FADFF`/`#0284C7`) replaces all green/teal for positive/online/done/waveforms/solo/meter-low; `--amber` (`#F6940F`/`#D97706`) for warnings/mute/meter-mid; `--red` (`#FC4232`/`#DC2626`) for playhead/arm/errors/meter-high; clip series `[--primary, --sky, --amber, --pink, --purple]`.
4. **Forbidden Patterns:** Zero `backdrop-filter`/blur, zero translucent surface overlays, zero `shadowBlur`/glow/pulse, zero arbitrary `box-shadow` outside `.shadow-card`, zero green hues (`75°-170°`), zero retired/foreign hexes, zero charts (`echarts` removed), zero hex/rgb/hsl/Tailwind palette classes outside the two token files.
5. **Typography & Canvases:** Self-hosted `InterVariable.ttf` with `tabular-nums` for telemetry/tables; `WaveformCanvas`, `SpectrogramCanvas`, and `MultitrackCanvas` read live theme CSS variables via `linearCanvasTheme.ts` on `theme-changed`.
