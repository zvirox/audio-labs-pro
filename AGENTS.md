# AGENTS.md — Audio Labs Pro Engineering & Design Contract

This document defines mandatory invariants for contributors and coding agents working in `audio-labs-pro`.

---

## 1. Locked Linear Design System (Final — Magenta Primary)

The **Linear-Style Solid Matte UI** is **FINAL and LOCKED**. Do not alter layout, spacing, rounding, or re-introduce glass/blur/charts.

- **Surfaces:** Flat, solid, opaque surfaces with `1px` hairline borders (`--bg-app`, `--bg-sidebar`, `--bg-surface`, `--bg-surface-2`, `--bg-active`, `--bg-hover`, `--bg-track`, `--border`, `--border-strong`). Card elevation uses `--shadow-card` (`.shadow-card`) only.
- **Primary Accent (Magenta) & Status Roles:** Magenta primary (`--primary` `#C026D3` dark / `#A21CAF` light, `--primary-text` `#E879F9` dark / `#A21CAF` light, `--primary-fg` `#FFFFFF`, `--primary-tint` `#331D36` dark / `#F8EFF9` light) lives strictly in `frontend/src/styles/app-accent.css`. Green and teal are banned everywhere; positive/healthy/online/done/up states and clip waveforms use `--sky`, `--sky-text`, and `--chip-sky`; warnings/Mute use `--amber`; errors/Playhead/Record-Arm use `--red`, `--red-text`, and `--chip-red`.
- **No Charts:** Audio Labs Pro does not include dashboard charts or `echarts` (`echarts-linear-theme.ts` and `echarts` must remain absent). Keep waveforms, level meters, spectrograms, and the timeline ruler/playhead rendered via native Canvas 2D / DOM tokens.
- **Strictly Forbidden:** `backdrop-filter`, `blur()`, `backdrop-blur-*`, translucent surface fills (`bg-white/*`, `bg-black/*`), arbitrary `rgba(...)`/`hsla(...)` (permitted *only* in `--scrim`, `--chart-fade-up`, `--chart-fade-down`, and `--shadow-card` inside `frontend/src/styles/linear-theme.css`), glow effects, inset highlights, arbitrary `box-shadow` outside `var(--shadow-card)`, surface gradients, raw hex/rgb/hsl or Tailwind palette classes outside `frontend/src/styles/linear-theme.css` and `frontend/src/styles/app-accent.css`, retired palette hexes, green hues (75°–170°), foreign app primaries, and pill-shaped cards/chips.
- **Window & Title Bar:** Wide 16:10 window (`1280×800` default, `1024×640` minimum, `decorations: false`, `transparent: false`, `shadow: true`). Identical `40px` `TitleBar` reading version from `crates/alp-desktop/tauri.conf.json`, with right-side theme toggle (`ui-theme` in `localStorage`), minimize, maximize/restore, and `#E5484D` (`--close-hover`) hover close.
- **Typography:** Self-hosted `InterVariable.ttf` only for UI text (`tabular-nums` for stats/tables). Keep `html { font-size: 16px; }`.
- **Required Gate:** Run `npm --prefix frontend run check:linear && npm --prefix frontend run build` before any UI change is considered complete.

---

## 2. Core Safety & Engineering Invariants

1. **Zero Audio-Thread Blocking:** File decoding, inference subprocesses (`alp-infer`), project/sample-block commits (`alp-core`), and IPC serialization must stay off real-time audio callbacks (`alp-engine`).
2. **Unsafe Code & Panics Forbidden in Desktop:** `alp-desktop` enforces zero `unsafe` blocks and returns structured `IpcError` DTOs.
3. **Consent-Gated Voice Cloning:** Zero-shot cloning requires explicit speaker attestation in `alp-voice` and `CloningConsentModal`.
4. **Repository Guards:** `./scripts/check-deps.sh`, `./scripts/check-docs.sh`, `./scripts/check-ui.sh`, and `./scripts/check-file-length.sh` must remain green.
5. **Locked Release Packaging:** Distribution packages are built exclusively via `packaging/nfpm.yaml` (`.deb`, `.rpm`, `.pkg.tar.zst` + `SHA256SUMS`).

---

## 3. Verification Commands

```bash
npm --prefix frontend run check:linear
npm --prefix frontend test
npm --prefix frontend run build
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
./scripts/check-deps.sh && ./scripts/check-docs.sh && ./scripts/check-ui.sh && ./scripts/check-file-length.sh
```
