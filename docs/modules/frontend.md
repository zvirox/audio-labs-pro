# `frontend` — React 19 Linear Desktop Shell

## Purpose

Implements the **final and locked Linear UI** webview frontend for `audio-labs-pro` using React 19, TypeScript, Tailwind CSS v4, Lucide icons, `cmdk`, and self-hosted `InterVariable.ttf` (chart-free).

## Architecture

- **`src/styles/linear-theme.css` & `src/styles/app-accent.css`:** Shared Tessera-sampled Linear palette (`#1C1C1C`/`#292929` dark, `#F8F9FA`/`#FFFFFF` light, zero green/teal) plus Audio Labs Pro **Magenta** primary accent (`#C026D3`/`#A21CAF`). All hex/rgb/hsl definitions live strictly inside these two files.
- **`src/tokens.css` & `src/index.css`:** Bundles self-hosted `Inter`, structural dimensions/motion vars, and Tailwind `@theme inline` utility bindings.
- **`src/components/linear/`:** Canonical Linear UI primitives (`TitleBar`, `ThemeToggle`, `Sidebar`, `Breadcrumb`, `LinearCard`, `LinearButton`, `LinearDialog`, `DataTable`, `StatCard`, `StatusPill`, `ProgressRow`, `RangeTabs`, `DonutLegend`).
- **`src/kit/` & `src/layout/`:** Adapter primitives (`Button`, `Card`, `Dialog`, `Input`, `Menu`, `Meter`, `Slider`, `Tabs`, `Toggle`, `Tooltip`) and workspace chrome (`PrimaryRail`, `SubtabTopBar`, `InspectorDrawer`, `DockedPlayer`, `panelLayout`).
- **`src/workspaces/`:** 9 workspaces (`DashboardWorkspace`, `VoicesWorkspace`, `GenerateWorkspace`, `MultitrackWorkspace`, `StoriesWorkspace`, `CapturesWorkspace`, `EffectsWorkspace`, `ModelsWorkspace`, `SettingsWorkspace`, plus `CloningConsentModal`).
- **`src/waveform/` & `src/timeline/`:** Theme-reactive 2D canvas renderers (`WaveformCanvas`, `SpectrogramCanvas`, `MultitrackCanvas`) consuming live tokens via `src/utils/linearCanvasTheme.ts`.
- **`scripts/check-linear.mjs`:** Automated static guard enforcing shared theme SHA-256 sync, exact Magenta accent block, zero green/retired/foreign colors, zero blur/translucency/glow/gradients, `.shadow-card` discipline, and zero `echarts` / chart imports.
