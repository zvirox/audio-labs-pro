# DESIGN.md — Audio Labs Pro Design System

**Version:** 2.1 (Locked Shared Linear Standard · Magenta Primary) · **Date:** 2026-11-26  
**Status:** FINAL & LOCKED — UI Style & Palette Frozen  
**Authority chain:** `docs/ADOPTION_PLAN.md` (Product Roadmap) → `docs/architecture/ARCHITECTURE.md` (Process Boundaries) → **this file (Locked Linear UI Law & Tokens)** → `docs/architecture/STACK.md` (Tech Stack)  
**Rule:** The Linear solid matte design system defined below is final and enforced automatically by `frontend/scripts/check-linear.mjs` (`npm --prefix frontend run check:linear`).

---

## 1. Visual Standard & Forbidden Patterns

- **Linear Matte Architecture:** Every card, panel, sidebar, modal, and button uses flat, solid, opaque surfaces with `1px` hairline borders. Card elevation uses `--shadow-card` (`.shadow-card`) only.
- **Themes:** Dual **Dark** (`data-theme="dark"`) and **Light** (`data-theme="light"`) themes persisted under `localStorage` key `ui-theme` (default follows OS `prefers-color-scheme`, applied inline in `frontend/index.html` before first paint). The toggle lives exclusively in the `40px` `TitleBar`.
- **Zero Green / Teal / Charts:** Green and teal (`--green`, `--teal`, green hues 75°–170°) are removed from the palette. Positive, healthy, online, done, and upward states use `--sky`, `--sky-text`, and `--chip-sky`. Audio Labs Pro contains no dashboard charts (`echarts` removed).
- **Forbidden Everywhere:**
  - Frosted glass, `backdrop-filter`, `blur()`, `backdrop-blur-*`, vibrancy / mica / acrylic.
  - Translucent surface fills (`bg-white/*`, `bg-black/*`). Alpha values are permitted *only* in `--scrim`, `--chart-fade-up`, `--chart-fade-down`, and `--shadow-card` inside `frontend/src/styles/linear-theme.css`.
  - Glows, inset highlights, and any `box-shadow` other than `var(--shadow-card)`.
  - Surface gradients (`bg-gradient-*`, `linear-gradient`, `radial-gradient`).
  - Raw hex, `rgb()`, `hsl()`, and Tailwind palette classes outside `frontend/src/styles/linear-theme.css` and `frontend/src/styles/app-accent.css`.
  - Retired palette hex values, green hues, foreign app primary accents, and any import of `echarts` or `echarts-linear-theme`.
  - macOS-style window traffic lights or left-side window buttons.
  - Pill-shaped cards, buttons, or chips (`rounded-full` restricted strictly to tiny status dots and avatars).

---

## 2. Color Tokens

### 2.1 Shared Palette (`frontend/src/styles/linear-theme.css`)

| Token | Dark | Light | Role |
| :--- | :--- | :--- | :--- |
| `--bg-app` | `#1C1C1C` | `#F8F9FA` | Window, title bar, breadcrumb bar, main area |
| `--bg-sidebar` | `#1C1C1C` | `#F3F4F6` | Sidebar and left rail |
| `--bg-surface` | `#1C1C1C` | `#FFFFFF` | Cards, panels, dialogs, command palette, floating HUD bar |
| `--bg-surface-2` | `#292929` | `#F3F4F6` | Inputs, version tag, count badge, status pill fill, icon chip, nested boxes, tooltip body |
| `--bg-active` | `#292929` | `#E5E7EB` | Active nav item, selected range tab |
| `--bg-hover` | `#232323` | `#EEF0F2` | Hovered nav item, title bar button hover |
| `--bg-track` | `#333333` | `#E5E7EB` | Switch off track, progress track |
| `--bg-bar-idle` | `#333333` | `#E5E7EB` | Shared idle bar token |
| `--border` | `#2E2E2E` | `#E5E7EB` | `1px` hairlines, table row dividers, ruler ticks/grid, default borders |
| `--border-strong` | `#363636` | `#D1D5DB` | Card hover border, dialog and tooltip borders, selected tab border |
| `--text` | `#FFFFFF` | `#111827` | Headings, KPI numbers, active tab, current breadcrumb |
| `--text-secondary` | `#CBCBCB` | `#111827` | Body text, labels, table rows, ghost buttons, nav items |
| `--text-muted` | `#B4B4B4` | `#5B6370` | Units, timestamps, table headers, version tag text, unselected tabs |
| `--text-faint` | `#939393` | `#818998` | Hints, expand icons, idle status dot |
| `--blue` | `#346AFE` | `#2563EB` | Plain series / accent option |
| `--sky` | `#2FADFF` | `#0284C7` | Positive: healthy, online, done, up; waveform on clips; Solo active; Meter low segments |
| `--sky-text` | `#2FADFF` | `#0369A1` | Positive delta text, positive icon/chip text |
| `--amber` | `#F6940F` | `#D97706` | Warning, Mute active, Meter mid segments, series color |
| `--pink` | `#FC669B` | `#DB2777` | Series / track clip color |
| `--purple` | `#6D59F9` | `#6D28D9` | Series / track clip color, plugin badges |
| `--orange` | `#FB6801` | `#EA580C` | Series color |
| `--red` | `#FC4232` | `#DC2626` | Error, critical, alert dot, Timeline Playhead, Record-Arm active, Meter high segments |
| `--red-text` | `#FF6B5E` | `#B91C1C` | Negative delta text and destructive button labels |
| `--gray` | `#4F4F4F` | `#868E9C` | Neutral series |
| `--chip-sky` | `#1F303C` | `#E0F2FE` | Positive delta chip, Done / Online / Ready chip |
| `--chip-red` | `#2D1F1F` | `#FEE2E2` | Negative delta chip, Danger chip |
| `--close-hover` | `#E5484D` | `#E5484D` | Window close button hover fill (icon stays `#FFFFFF`) |
| `--scrim` | `rgba(0,0,0,0.7)` | `rgba(0,0,0,0.4)` | Dialog backdrop (solid dim, zero blur) |
| `--shadow-card` | `0 12px 28px -6px rgba(0,0,0,0.5)` | `0 1px 2px rgb(16 24 40 / 5%)` | Allowed card elevation shadow (`.shadow-card`) |

### 2.2 App Primary Accent: Magenta (`frontend/src/styles/app-accent.css`)

| Token | Dark | Light | Role |
| :--- | :--- | :--- | :--- |
| `--primary` | `#C026D3` | `#A21CAF` | Primary button fill, switch on, slider/fader fill, selected clip border, progress fill |
| `--primary-hover` | `#A921BA` | `#8F199A` | Primary button hover |
| `--primary-fg` | `#FFFFFF` | `#FFFFFF` | Primary button text |
| `--primary-text` | `#E879F9` | `#A21CAF` | Links, 2px focus ring, selected tab text & underline, active nav icon |
| `--primary-tint` | `#331D36` | `#F8EFF9` | Assigned / info chip fill |

Clip and track series colors (chart-free): `--primary`, `--sky`, `--amber`, `--pink`, `--purple`.

---

## 3. Window, Typography & Layout Contract

- **Window (`crates/alp-desktop/tauri.conf.json`):** `1280×800` default (16:10), `1024×640` minimum, `decorations: false`, `transparent: false`, `resizable: true`, `shadow: true`, compositor blur disabled.
- **Typography:** Self-hosted `InterVariable.ttf` for UI text (`12 / 13 / 14 / 16 / 20 / 28 px`, `tabular-nums` for telemetry metrics). Monospace restricted to timecodes, keycaps, and log/path readouts.
- **Minimal Rounding:** Cards & dialogs `8px` (`rounded-lg`), buttons & inputs `6px` (`rounded-md`), chips, clips, tags & tooltips `4px` (`rounded`), status dots/avatars `rounded-full`.
- **Shared Audio Displays:** Canvas waveforms (`WaveformCanvas`), STFT spectrograms (`SpectrogramCanvas`), non-linear timeline (`MultitrackCanvas`), and discrete 20-segment meters (`Meter`) use live CSS variables via `linearCanvasTheme.ts` without charts or ECharts.

---

*UI Design Locked — Zvirox / Audio Labs Pro.*
