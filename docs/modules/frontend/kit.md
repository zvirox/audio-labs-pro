# `frontend/src/kit` & `frontend/src/components/linear`

Unified opaque Linear UI primitive components powered by shared tokens + Magenta primary (`--primary`). No translucent fills, compositor blur, green hues, charts, or pill-shaped buttons are permitted.

## Linear Primitives (`src/components/linear/`)

- `TitleBar` — 40 px custom frameless header (20 px logo mark, app title, version chip read from `tauri.conf.json`, clean drag region, sole `ThemeToggle`, divider, minimize/maximize/`#E5484D` close).
- `ThemeToggle` — Single light/dark switcher bound to `ui-theme`, dispatching `theme-changed` events cleanly with zero transition lag.
- `Sidebar` — Collapsible (`240 px` / `56 px`) navigation rail (`--bg-sidebar`) with 36 px items (`--bg-active` + `--primary-text` icon when active), workspace switcher, command palette trigger, and shortcuts button.
- `Breadcrumb` — Top workspace breadcrumb (`Audio Labs Pro / Workspace / Subtab`), Native/Preview status pill, revision counter, dirty badge, split panel toggles, and Inspector toggle.
- `LinearCard` — Solid layered container (`level 1 | 2 | 3`) with `8 px` (`rounded-lg`) corners, `1 px` hairline border, and `.shadow-card`.
- `LinearButton`, `LinearDialog` (`bg-scrim`), `DataTable`, `StatCard` (number + unit + delta tag, no sparklines), `StatusPill` (`sky` positive, `amber` warning, `red` danger, `primary` info, `faint` idle), `ProgressRow`, `RangeTabs`, `DonutLegend`.

## Kit Primitives (`src/kit/`)

- `Button` / `Input` / `Tabs` / `Toggle` / `Slider`: Magenta `--primary` / `--primary-text` active states, `--bg-track` switch/slider tracks, `6 px` (`rounded-md`) corners.
- `Card` & `Dialog`: Delegate to `LinearCard` and `LinearDialog` (`bg-scrim` dim, zero blur).
- `Meter`: Discrete flat 20-segment LED strip (`--sky` low -> `--amber` mid -> `--red` high), zero green, zero gradients or glow.
- `Menu` & `Tooltip`: Opaque surfaced overlays with `--border-strong` boundaries and `.shadow-card`.
