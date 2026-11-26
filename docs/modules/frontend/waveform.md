# `frontend/src/waveform`

Multi-resolution audio peak & STFT spectrogram Canvas 2D renderers:

- **`WaveformCanvas.tsx`:** Renders multi-resolution min/max peak pyramids in flat Linear Sky (`--sky`: `#2FADFF` dark / `#0284C7` light) on `--bg-surface-2` with zero glow or gradients, dotted `--border` ruler ticks, selection & loop overlays (`--chip-sky` / `--primary-tint` + `--primary`), and a solid `--red` (`#FC4232`/`#DC2626`) playhead. Automatically redraws on `theme-changed`.
- **`SpectrogramCanvas.tsx`:** Renders STFT magnitude matrices with calibrated Magma/Inferno/Viridis LUTs, logarithmic frequency grid lines (`--border-strong`), interactive 2D time-frequency selection rectangles (`--chip-sky` / `--sky`), corner grab handles, and a solid floating HUD bar (`--bg-surface`, `--border-strong`) displaying `Δt`, frequency range, `fc`, and `Q`.
