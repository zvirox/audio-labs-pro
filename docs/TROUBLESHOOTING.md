# Audio Labs Pro — Troubleshooting & Platform Guide

Guidance on configuring Linux audio systems, Wayland compositors, and GPU acceleration for Audio Labs Pro.

---

## 1. Linux Audio: PipeWire, PulseAudio & ALSA

Audio Labs Pro uses high-performance audio streams for low-latency playback and crash-resilient recording.

### Recommended Audio Setup: PipeWire
Modern distributions (Fedora, Ubuntu 22.04+, Arch, Debian 12+) use PipeWire by default. Audio Labs Pro connects seamlessly via PipeWire's ALSA/PulseAudio emulation layer.
- Verify PipeWire status:
  ```bash
  systemctl --user status pipewire pipewire-pulse
  ```
- If experiencing xruns (audio dropouts or buffer underruns) under heavy load, check your quantum buffer size:
  ```bash
  pw-metadata -n settings 0 clock.quantum 1024
  ```

### Raw ALSA Fallback
If PipeWire is not present, Audio Labs Pro falls back to direct ALSA devices.
- Ensure your user belongs to the `audio` group:
  ```bash
  sudo usermod -aG audio $USER
  ```
- Test ALSA capture and playback devices:
  ```bash
  arecord -l
  aplay -l
  ```

---

## 2. Wayland & Compositor Blur Effects

Audio Labs Pro features a fluid glass aesthetic with compositor background blur.

### KDE Plasma (Wayland & X11)
- Plasma's KWin compositor natively supports background blur on Wayland via `org_kde_kwin_blur` protocols.
- Audio Labs Pro automatically detects KWin and activates hardware-accelerated blur.

### GNOME Shell (Wayland)
- GNOME Shell does not expose an unprivileged client-side blur protocol by default.
- When running under GNOME, Audio Labs Pro smoothly falls back to **Tinted Glass Mode**, providing high-contrast readability without requiring non-standard shell extensions.

### wlroots Compositors (Sway, Hyprland, River)
- Hyprland enables window blur through compositor-level window rules. Add the following to your `hyprland.conf`:
  ```ini
  windowrulev2 = blur, class:^(audio-labs-pro)$
  ```

---

## 3. NVIDIA Proprietary Drivers

When running Audio Labs Pro on systems with proprietary NVIDIA drivers:
- Ensure the `nvidia-drm.modeset=1` kernel parameter is enabled for smooth Wayland buffer synchronization.
- If you notice window flickering with Wayland and NVIDIA, launch the AppImage or binary with:
  ```bash
  WINIT_UNIX_BACKEND=x11 audio-labs-pro
  ```
  This forces X11 fallback through XWayland while maintaining 60 fps rendering.

---

## 4. Voice Model Inference & Memory

- **Lightweight Built-in Acoustic:** Requires 0 MB download and minimal RAM (< 100 MB). Fully supported on any dual-core CPU.
- **Kokoro & Kitten Neural Models:** Require ~300 MB RAM and run with sub-second latency on modern CPUs.
- **GPU Acceleration:** Audio Labs Pro probes `/dev/dri/renderD*` nodes. If available, neural models can leverage hardware acceleration.
- If inference daemon crashes due to memory limits, the main workstation UI remains completely isolated and displays a recoverable warning toast.
