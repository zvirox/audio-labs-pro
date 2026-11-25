# `frontend` — React 19 Linear Desktop Shell

## Purpose

Implements the **final and locked Linear UI** webview frontend for `audio-labs-pro` using React 19, TypeScript, Tailwind CSS v4, ECharts, Lucide icons, `cmdk`, and self-hosted `InterVariable.ttf`.

## Architecture

- **`src/tokens.css` & `src/index.css`:** Canonical CSS variables for Dark (`#050505` / `#111213`) and Light (`#F3F4F6` / `#FFFFFF`) themes, `--accent-blue: #3355FF`, precomputed opaque `--tint-*` tokens, zero compositor blur, zero drop shadows in dark mode, and bundled `Inter` `@font-face`.
- **`src/components/linear/`:** Canonical Linear UI primitives (`TitleBar`, `ThemeToggle`, `Sidebar`, `Breadcrumb`, `LinearCard`, `LinearButton`, `LinearDialog`, `DataTable`, `StatCard`, `StatusPill`, `ProgressRow`, `RangeTabs`, `DonutLegend`, `echarts-linear-theme.ts`).
- **`src/kit/` & `src/layout/`:** Adapter primitives (`Button`, `Card`, `Dialog`, `Input`, `Menu`, `Meter`, `Slider`, `Tabs`, `Toggle`, `Tooltip`) and workspace chrome (`PrimaryRail`, `SubtabTopBar`, `InspectorDrawer`, `DockedPlayer`, `panelLayout`).
- **`src/workspaces/`:** 9 workspaces (`DashboardWorkspace`, `VoicesWorkspace`, `GenerateWorkspace`, `MultitrackWorkspace`, `StoriesWorkspace`, `CapturesWorkspace`, `EffectsWorkspace`, `ModelsWorkspace`, `SettingsWorkspace`, plus `CloningConsentModal`).
- **`src/waveform/` & `src/timeline/`:** Theme-reactive 2D canvas renderers (`WaveformCanvas`, `SpectrogramCanvas`, `MultitrackCanvas`) consuming live tokens via `src/utils/linearCanvasTheme.ts`.
- **`scripts/check-linear.mjs`:** Automated static guard failing on `backdrop-filter`, blur, translucent fills (`rgba()`, `hsla()`, `/opacity`), glow/pulse effects, surface gradients, dark shadows, or off-token raw hex colors.

## Theme Lifecycle

Theme preference (`dark` | `light`) is persisted under `localStorage["ui-theme"]`, applied synchronously before first paint in `frontend/index.html` via `data-theme="light|dark"` on `<html>`, and toggled exclusively from the `TitleBar` button or `Ctrl+D`.
