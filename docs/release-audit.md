# Release & Packaging Audit — Audio Labs Pro

**Verified State:** `v0.1.0-beta.6`  
**Repository:** `audio-labs-pro` (`https://github.com/zvirox/audio-labs-pro`)

---

## 1. Automated Workflows (`.github/workflows/`)

- **`.github/workflows/ci.yml`:** Runs on push/PR to `main` (`cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --workspace`, `npm run check:linear`, `npm test`, `npm run build`, and `./scripts/check-{deps,docs,ui,file-length}.sh`).
- **`.github/workflows/release.yml`:** Runs on `v*` tag pushes on `ubuntu-22.04` (compatible with Ubuntu 22.04 `glibc 2.35+`, Ubuntu 24.04, Fedora, and Arch). Builds release binaries (`audio-labs-pro`, `alp-cli`, `alp-infer`, `alp-mcp`) and delegates multi-distro packaging to `nfpm`.

---

## 2. Unified `nfpm` Packaging (`packaging/nfpm.yaml`)

Legacy manual `deb/`, `rpm/`, `flatpak/` directories were retired in M8 in favor of a single declarative `packaging/nfpm.yaml`:
- `.deb` (Debian / Ubuntu)
- `.rpm` (Fedora / RHEL / openSUSE)
- `.pkg.tar.zst` (Arch Linux / CachyOS)

Desktop metadata installed under `/usr/share/`:
- `packaging/audio-labs-pro.desktop` -> `/usr/share/applications/audio-labs-pro.desktop`
- `packaging/audio-labs-pro.xml` -> MIME integration for `.alpproj`
- `packaging/icons/hicolor/...` -> `audio-labs-pro.png`

---

## 3. Version & Binary Matrix (Aligned)

| Manifest / Target | Value |
|---|---|
| Workspace `Cargo.toml` | `0.1.0-beta.6` |
| `crates/alp-desktop/tauri.conf.json` | `0.1.0-beta.6` (`identifier: "io.github.zvirox.audio-labs-pro"`) |
| `frontend/package.json` | `0.1.0-beta.6` (TitleBar reads `__APP_VERSION__` from `tauri.conf.json`) |
| Primary GUI binary | `audio-labs-pro` (`crates/alp-desktop/Cargo.toml`) |
| Companion binaries | `alp-cli`, `alp-infer`, `alp-mcp` |
