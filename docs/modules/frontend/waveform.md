# `frontend/src/waveform`

Multi-resolution audio peak & STFT spectrogram Canvas 2D renderers:

- **`WaveformCanvas.tsx`:** Renders multi-resolution min/max peak pyramids in crisp flat Linear Sky (`--accent-sky`) on `--bg-surface-2` with zero glow or gradients, dotted ruler ticks, selection & loop overlays (`--tint-sky` / `--tint-purple`), and a solid `--accent-red` playhead. Automatically redraws on `theme-changed`.
- **`SpectrogramCanvas.tsx`:** Renders STFT magnitude matrices with calibrated Magma/Inferno/Viridis LUTs, logarithmic frequency grid lines (`--border-strong`), interactive 2D time-frequency selection rectangles, corner grab handles, and an opaque solid HUD overlay displaying `Δt`, frequency range, and `fc`.
