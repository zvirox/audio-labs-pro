# Troubleshooting Guide

Common diagnostics for Audio Labs Pro on Linux desktop sessions.

---

## 1. WebKitGTK / Window Rendering Issues

Audio Labs Pro uses a solid, opaque **Linear UI** window (`transparent: false`, compositor blur permanently disabled). No KWin blur rules or compositor transparency tweaks are required.

- If you observe blank frames on certain NVIDIA Wayland drivers with WebKitGTK, test explicit DMA-BUF opt-out:
  ```bash
  WEBKIT_DISABLE_DMABUF_RENDERER=1 audio-labs-pro
  ```
- If the window opens smaller than comfortable, reset or resize freely above the enforced minimum of `1024x640` (default launch geometry is `1280x800`).

---

## 2. Audio Device / PipeWire / ALSA Dropouts

1. Open **Preferences (`Ctrl+7`) -> Audio Hardware & ALSA** to verify the default PipeWire or ALSA sink/source.
2. Confirm PipeWire is active (`systemctl --user status pipewire pipewire-pulse`).
3. Ensure sample rates match across input and output streams (`48.0 kHz` recommended).

---

## 3. Theme Preference Not Updating

- Click the sun/moon icon in the top-right **TitleBar** or press `Ctrl+D`.
- Theme state persists under `localStorage["ui-theme"]` (`dark` or `light`) alongside `data-theme` on the document root so it paints immediately on launch without flash.

---

## 4. MCP Socket & Starship Control Diagnostics

- **Live Desktop MCP Socket:** When **Allow MCP** is active in Preferences, the host listens at:
  ```bash
  $XDG_RUNTIME_DIR/alp/control.sock
  ```
- **Starship Control Bridge:** Registers health and capability metadata at:
  ```bash
  $XDG_RUNTIME_DIR/starship/bridges/alp.sock
  ```
- If an external MCP tool returns a permission error, verify your selected **Permission Tier** (`Observe`, `Edit`, or `Export`) in **Preferences -> MCP Agents & Tiers**.

---

## 5. Neural Inference Subprocess (`alp-infer`)

- Ensure `alp-infer` is installed alongside `audio-labs-pro` in `/usr/bin/alp-infer` (or built via `cargo build --workspace` during local development).
- Model files downloaded from HuggingFace are verified by SHA-256 checksum inside `alp-voice::sha256` before activation.
