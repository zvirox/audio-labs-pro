# Audio Labs Pro Documentation

Welcome to the engineering, architectural, and user documentation for **Audio Labs Pro** (`v0.1.0-beta.1`) — an offline-first Linux workstation combining multitrack audio production, neural speech synthesis & cloning, and authenticated AI MCP automation inside a locked Tauri v2 **Linear UI** desktop shell.

---

## Core Roadmaps & Status

- [**MASTER CHECKLIST**](../MASTER_CHECKLIST.md) — Verified inventory of implemented crates, Linear UI completion, and remaining feature backlog
- [**Adoption & Feature Plan**](ADOPTION_PLAN.md) — Phased implementation plan separating finished M0–M8/Phases 0–13 work from Phases 14–18
- [**Status Report**](../ALP_STATUS_REPORT.md) — Current codebase and release baseline (`v0.1.0-beta.1`)
- [**Release & Packaging Audit**](release-audit.md) — Unified `nfpm` packaging configuration (`.deb`, `.rpm`, `.pkg.tar.zst`)

---

## Architecture & Locked UI Standard

- [**System Architecture**](architecture/ARCHITECTURE.md) — 12-crate Rust DAG, isolated `alp-infer`, `alp-session`, `alp-desktop`, and `frontend/`
- [**Technology Stack**](architecture/STACK.md) — Rust 2021/2024 + Tauri v2 + React 19 + Tailwind CSS v4 + ECharts + `nfpm`
- [**UI Migration & Locked Standard**](architecture/UI_MIGRATION.md) — Permanent locked Linear Design System specification and M0–M8 completion record
- [**Architecture Decision Records (ADRs)**](adr/) — Architectural history (`0001`–`0029`)

---

## User & Operator Manuals

- [**User Guide**](USER_GUIDE.md) — Installing Linux packages, navigating Linear workspaces, cloning safety, and MCP controls
- [**Keyboard Shortcuts**](KEYBOARD_SHORTCUTS.md) — Global, transport, and multitrack hotkeys
- [**Troubleshooting**](TROUBLESHOOTING.md) — PipeWire/ALSA devices, WebKitGTK rendering, theme storage, and MCP socket diagnostics
- [**Ethics & Responsible Cloning Policy**](ETHICS.md) — Consent attestation requirements

---

## MCP & Starship Integration

- [**Agent Guide**](mcp/AGENT_GUIDE.md) · [**Tool Catalog (68 Tools)**](mcp/TOOLS.md) · [**Security Tiers**](mcp/SECURITY.md) · [**Client Setup**](mcp/CLIENT_SETUP.md)

---

## Rust & Frontend Module Index

- **Rust Modules:** `docs/modules/alp-{api,cli,core,desktop,dsp,engine,infer,mcp,media,plugin,session,voice}.md`
- **Frontend Modules:** [frontend.md](modules/frontend.md), [kit.md](modules/frontend/kit.md), [layout.md](modules/frontend/layout.md), [workspaces.md](modules/frontend/workspaces.md), [timeline.md](modules/frontend/timeline.md), [waveform.md](modules/frontend/waveform.md)
