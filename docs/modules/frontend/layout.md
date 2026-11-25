# `frontend/src/layout`

## Shell Geometry

- **Window:** Default `1280x800` (16:10), minimum `1024x640`, frameless solid opaque window (`decorations: false`, `transparent: false`, `shadow: true`).
- **`TitleBar` (40 px):** Owns window drag, double-click maximize, `ThemeToggle`, and window controls (`WindowControls.tsx` retired).
- **`PrimaryRail` / `Sidebar` (240 px expanded, 56 px collapsed):** Workspace switcher across Dashboard, Voice Studio, Multitrack DAW, Audiobook Lab, Video Dubbing, Batch & Repair, DSP & Effects, and Preferences.
- **`Breadcrumb` + `SubtabTopBar`:** Dynamic breadcrumb trail, session revision state, workspace subtab switcher, and floating-panel controls.
- **`InspectorDrawer` (280 px right drawer):** Toggleable (`Ctrl+B`) channel gain/pan strip, Mute/Solo/Arm buttons, segmented output meter, and loudness normalize toggle.
- **`DockedPlayer` (56 px bottom transport):** Global Play/Pause/Stop/Record/Loop transport, timecode scrub slider, discrete segmented stereo peak meters, sample rate badge, and master gain slider.
- **`panelLayout.ts`:** Window-resize bounds clamping and overlap auto-tidying (`tidyPanels`, `clampPanels`, `hasOverlap`).
