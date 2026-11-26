# Audio Labs Pro — Color & Chart Migration Audit

Generated prior to applying the Tessera-aligned shared palette (`linear-theme.css`) and Magenta primary accent (`app-accent.css`).

---

## 1. Raw Hex, RGB/HSL, Tailwind Palette, Arbitrary Shadow & Chart Hits Outside Token Files

| File | Line | Existing Usage | Action / Target Role |
| --- | ---: | --- | --- |
| `frontend/src/tokens.css` | 16–154 | Old Linear dark `#050505`/`#111213`/`#3355FF`, green `#22C55E`, teal `#1FB5A5`, raw hex/tint tokens | Move all colors into byte-identical `src/styles/linear-theme.css` + Magenta `src/styles/app-accent.css`; strip all hex/rgb/hsl from `tokens.css` |
| `frontend/src/components/linear/echarts-linear-theme.ts` | 14–31 | Raw ECharts hex palette including `#3355FF`, `#1FB5A5` (`teal`), `#111213`, `#F5F5F6` | **Remove file** (Section 5: no charts in ALP) |
| `frontend/src/components/linear/index.ts` | 14 | Exports `./echarts-linear-theme` | Remove export |
| `frontend/package.json` | 19 | `"echarts": "^6.1.0"` | Remove `echarts` dependency |
| `frontend/src/workspaces/DashboardWorkspace.tsx` | 2, 62–70 | `import * as echarts from 'echarts'` + ECharts telemetry chart + `RangeTabs` | Remove ECharts chart completely; replace with non-chart audio engine telemetry & pipeline status cards |
| `frontend/src/workspaces/CapturesWorkspace.tsx` | 92 | `text-red-400` (Tailwind palette class) | Replace with `text-red-text` / `text-red` |
| `frontend/src/workspaces/CapturesWorkspace.tsx` | 127 | `text-red-500` (Tailwind palette class) | Replace with `text-red` |
| `frontend/src/workspaces/SettingsWorkspace.tsx` | 401 | `hover:text-cyan-200` (Tailwind palette class) | Replace with `hover:text-primary-text` |
| `frontend/src/waveform/SpectrogramCanvas.tsx` | 506 | `hover:text-rose-400` (Tailwind palette class) | Replace with `hover:text-red-text` |
| `frontend/src/timeline/MultitrackCanvas.tsx` | 340 | `ctx.fillStyle = \`rgb(${rCol}, ${gCol}, ${bCol})\`;` | Replace with `ImageData` pixel buffer rendering (zero `rgb()` CSS strings) |
| `frontend/src/components/linear/LinearCard.tsx` | 24–26 | `shadow-[var(--shadow-card)]` | Replace with `.shadow-card` |
| `frontend/src/components/linear/LinearButton.tsx` | 26 | `shadow-[var(--shadow-card)]` | Replace with `.shadow-card` |
| `frontend/src/components/CommandPalette.tsx` | 68 | `shadow-[var(--shadow-card)]` | Replace with `.shadow-card` |
| `frontend/src/kit/Button.tsx` | 49 | `shadow-[var(--shadow-card)]` | Replace with `.shadow-card` |
| `frontend/src/kit/Menu.tsx` | 62 | `shadow-[var(--shadow-card)]` | Replace with `.shadow-card` |
| `frontend/src/kit/Tooltip.tsx` | 52 | `shadow-[var(--shadow-card)]` | Replace with `.shadow-card` |
| `frontend/src/panels/PanelContainer.tsx` | 82 | `shadow-[var(--shadow-card)]` | Replace with `.shadow-card` |

---

## 2. Green / Teal & Former Primary / Surface Role Swaps Across `frontend/src/`

| Category | Former Tokens / Classes | New Target Token / Role | Affected Files |
| --- | --- | --- | --- |
| **Green / Teal** (**REMOVED**) | `--accent-green`, `--tint-green`, `--accent-teal`, `--tint-teal`, `--color-success`, `bg-[var(--accent-green)]`, `bg-[var(--tint-green)]`, `text-[var(--accent-green)]` | `--sky`, `--sky-text`, `--chip-sky` (`bg-chip-sky text-sky-text`, `bg-sky`); removed teal from series | `Meter.tsx`, `StatusPill.tsx`, `linearCanvasTheme.ts`, `MultitrackCanvas.tsx`, `EffectsWorkspace.tsx`, `VoicesWorkspace.tsx`, `ModelsWorkspace.tsx`, `StoriesWorkspace.tsx`, `SettingsWorkspace.tsx`, `StatCard.tsx`, `DashboardWorkspace.tsx` |
| **App Primary (Blue -> Magenta)** | `--accent-blue`, `--accent-blue-hover`, `--tint-blue`, `--color-accent`, `--text-on-accent` | `--primary` (`#C026D3`/`#A21CAF`), `--primary-hover`, `--primary-fg` (`#FFFFFF`), `--primary-text` (`#E879F9`/`#A21CAF`), `--primary-tint` (`#331D36`/`#F8EFF9`) | `Button.tsx`, `LinearButton.tsx`, `Toggle.tsx`, `Slider.tsx`, `Tabs.tsx`, `Input.tsx`, `TitleBar.tsx`, `Sidebar.tsx`, `StatCard.tsx`, `StatusPill.tsx`, `ProgressRow.tsx`, `DockedPlayer.tsx`, `InspectorDrawer.tsx`, `CommandPalette.tsx`, `CloningConsentModal.tsx`, `CapturesWorkspace.tsx`, `EffectsWorkspace.tsx`, `ModelsWorkspace.tsx`, `SettingsWorkspace.tsx`, `StoriesWorkspace.tsx`, `VoicesWorkspace.tsx`, `MultitrackWorkspace.tsx`, `MultitrackCanvas.tsx`, `SpectrogramCanvas.tsx`, `linearCanvasTheme.ts`, `App.tsx` |
| **Sidebar / Rail surface** | `--bg-surface` (shared with cards) | `--bg-sidebar` (`bg-sidebar`), active item `bg-active text-fg border-border-strong` with `--primary-text` icon, hover `bg-hover` | `Sidebar.tsx`, `TitleBar.tsx`, `PrimaryRail.tsx` |
| **Controls / Hover / Track / Idle** | `--bg-surface-2` & `--bg-bar-idle` overloaded 4 roles | Split strictly: `--bg-surface-2` (inputs/tags/badges/icon chips), `--bg-active` (active nav/selected tab), `--bg-hover` (hovered nav/titlebar buttons), `--bg-track` (switch off / progress track) | `Toggle.tsx`, `Slider.tsx`, `ProgressRow.tsx`, `RangeTabs.tsx`, `Tabs.tsx`, `Meter.tsx`, `Sidebar.tsx`, `TitleBar.tsx`, `LinearButton.tsx`, `Button.tsx`, `PanelContainer.tsx` |
| **Secondary Body Text vs Headings** | `--text` used for both headings and body/nav labels | `--text` (headings, KPI numbers, active labels) vs `--text-secondary` (body text, table rows, nav labels, ghost buttons) vs `--text-muted` / `--text-faint` | `Sidebar.tsx`, `DataTable.tsx`, `LinearCard.tsx`, `LinearDialog.tsx`, `Button.tsx`, `LinearButton.tsx` |
| **Modal Scrim** | `bg-[var(--bg-app)] opacity-80` | `bg-scrim` (`var(--scrim)`) | `LinearDialog.tsx`, `CommandPalette.tsx` |
| **Meter Strips** | `--accent-green` / `--accent-amber` / `--accent-red` | `--sky` (low) / `--amber` (mid) / `--red` (high) | `Meter.tsx` |
| **Mixer / Multitrack** | M/S/R pills & series `[blue, sky, teal, purple, orange]` | Mute `--amber`, Solo `--sky`, Arm `--red`, Playhead `--red`, Waveform `--sky`, Selected border `--primary`, Series `[--primary, --sky, --amber, --pink, --purple]` | `MultitrackCanvas.tsx`, `WaveformCanvas.tsx`, `linearCanvasTheme.ts` |
