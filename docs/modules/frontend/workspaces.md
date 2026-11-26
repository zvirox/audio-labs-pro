# `frontend/src/workspaces`

9 Linear-styled workspaces built on `LinearCard`, `StatCard`, `DataTable`, `Card`, `Button`, `Slider`, `Toggle`, `Tabs`, and Canvas 2D views (chart-free):

1. **`DashboardWorkspace`:** Top KPI `StatCard` row (`28 px` semibold `tabular-nums`, no sparklines), Audio Engine & Subsystems status grid, `Subsystem Allocation` `ProgressRow` bars + `DonutLegend`, and clickable `DataTable` launching every studio module.
2. **`VoicesWorkspace` & `CloningConsentModal`:** Zero-shot cloning profile list, mandatory written speaker consent attestation dialog, voice similarity and preview player.
3. **`GenerateWorkspace`:** Text-to-speech synthesis prompt workbench, speech speed/pitch sliders, and rendered take history.
4. **`MultitrackWorkspace`:** Arranger toolbar (`Undo`, `Redo`, `Split`, `Delete`, `Snap`, `Ripple`, global `Wave | Spectral | Split` switcher), zoom controls, full-bleed `MultitrackCanvas`, and track/clip footer bar.
5. **`StoriesWorkspace`:** Long-form audiobook chapter scripting, speaker voice tagging, and synthesis queue status.
6. **`CapturesWorkspace`:** Live microphone capture timer, VAD word-level transcript selector chips, and jump-to-word seeking.
7. **`EffectsWorkspace`:** Cleanup stages banner, parametric EQ / compression / reverb controls, a native Canvas 2D magnitude/phase plot fed by real biquad telemetry, and discovered CLAP / LV2 plugins list.
8. **`ModelsWorkspace`:** Built-in & downloadable ONNX/GGUF model catalog with SPDX license tags, sizes, and local SHA-256 verification status.
9. **`SettingsWorkspace`:** Audio hardware devices (PipeWire/ALSA), **Window & Surface Architecture** (solid opaque Linear mode readout; theme controlled via TitleBar/`Ctrl+D`), MCP AI security tiers (`Observe` / `Edit` / `Export`), client approval prompts, live activity feed, and delivery loudness presets.
