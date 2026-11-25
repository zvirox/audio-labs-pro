# `frontend/src/kit` & `frontend/src/components/linear`

Unified opaque Linear UI primitive components. No translucent fills, compositor blur, surface gradients, or pill-shaped buttons are permitted.

## Linear Primitives (`src/components/linear/`)

- `TitleBar` — 40 px custom frameless header (20 px logo mark, app title, version chip read from `tauri.conf.json`, clean drag region, sole `ThemeToggle`, divider, minimize/maximize/`#E5484D` close).
- `ThemeToggle` — Single light/dark switcher bound to `ui-theme`, dispatching `theme-changed` events to canvas components and ECharts.
- `Sidebar` — Collapsible (`240 px` / `56 px`) navigation rail with 36 px items, workspace switcher, command palette trigger, and shortcuts button (contains no duplicate theme toggle).
- `Breadcrumb` — Top workspace breadcrumb (`Audio Labs Pro / Workspace / Subtab`), Native/Preview pill, revision counter, dirty badge, split panel toggles, and Inspector toggle.
- `LinearCard` — Solid layered container (`level 1 | 2 | 3`) with `8 px` (`rounded-lg`) corners and `1 px` hairline border (`--shadow-card`: `none` in dark, subtle 1 px hairline in light).
- `LinearButton`, `LinearDialog`, `DataTable`, `StatCard`, `StatusPill`, `ProgressRow`, `RangeTabs`, `DonutLegend`, `echarts-linear-theme.ts`.

## Kit Primitives (`src/kit/`)

- `Button` / `Input` / `Tabs` / `Toggle` / `Slider`: `6 px` (`rounded-md`) inputs/controls and `4 px` (`rounded`) compact chips.
- `Card` & `Dialog`: Delegate to `LinearCard` and `LinearDialog` with solid dimmed scrim (zero blur).
- `Meter`: Discrete flat 20-segment LED strip (`--accent-green` -> `--accent-amber` -> `--accent-red`), zero gradients or glow.
- `Menu` & `Tooltip`: Opaque surfaced overlays with `--border-strong` boundaries.
