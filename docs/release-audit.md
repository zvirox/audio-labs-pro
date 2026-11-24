# Release & Packaging Audit — Audio Labs Pro

**Audited:** 2026-11-24  
**Auditor:** Prakash / Antigravity  
**Repository:** `audio-labs-pro` (`https://github.com/zvirox/audio-labs-pro`)

---

## 1. Workflows in `.github/workflows/`

* **`.github/workflows/ci.yml`**:
  * **Trigger:** `push` / `pull_request` on `main`.
  * **Jobs:** `workspace` (test, check, clippy), `frontend` (build, test).
* **`.github/workflows/release.yml`**:
  * **Trigger:** `push` on tags matching `v*` or `workflow_dispatch`.
  * **Jobs:**
    1. `packages`: builds `.deb` and `.rpm` using manual `dpkg-deb` and `rpmbuild`.
    2. `flatpak`: compiles workspace in Freedesktop SDK building `Audio-Labs-Pro-${VERSION}-x86_64.flatpak`.
    3. `publish`: aggregates artifacts, calculates `sha256sums.txt`, and creates GitHub Release via `gh release create`.

---

## 2. Existing Packaging Files & Configuration

* **`crates/alp-desktop/tauri.conf.json`**:
  * `"productName": "Audio Labs Pro"`
  * `"identifier": "io.github.zvirox.audio-labs-pro"`
* **`packaging/` directory contents:**
  * `packaging/desktop/io.github.zvirox.AudioLabsPro.desktop`
  * `packaging/desktop/io.github.zvirox.AudioLabsPro.metainfo.xml`
  * `packaging/mime/io.github.zvirox.AudioLabsPro.xml`
  * `packaging/icons/hicolor/...`
  * `packaging/deb/control`
  * `packaging/rpm/audio-labs-pro.spec`
  * `packaging/flatpak/io.github.zvirox.AudioLabsPro.yaml`
  * `packaging/flatpak/throwaway-tauri-derisk.yaml`

---

## 3. Existing Tags and Release Assets

* **Tags:** `v0.0.1-alpha.1` through `v0.0.1-alpha.9`
* **Latest Release (`v0.0.1-alpha.9`):**
  * `Audio-Labs-Pro-0.0.1-alpha.9-x86_64.flatpak` (15.21 MiB)
  * `audio-labs-pro-0.0.1.alpha.9-1.x86_64.rpm` (27.71 MiB)
  * `audio-labs-pro_0.0.1-alpha.9_amd64.deb` (19.96 MiB)
  * `sha256sums.txt` (323 B)
  * *(Notice: No Arch `pkg.tar.zst` was built)*

---

## 4. Version Alignment Check

| Location | Version | Matches Latest Tag (`v0.0.1-alpha.9`)? |
|---|---|---|
| `Cargo.toml` (workspace) | `0.1.0` | **Mismatch** (`0.1.0` vs tag `v0.0.1-alpha.9`) |
| `crates/alp-desktop/tauri.conf.json` | `0.1.0` | **Mismatch** |
| `frontend/package.json` | `1.0.0` | **Mismatch** (`1.0.0`) |

*Note: The upcoming release will reconcile version across manifests and tags.*

---

## 5. Naming Matrix & Identifier Alignment

* **GitHub Repository Name:** `audio-labs-pro`
* **Cargo Binary Name:** `alp-desktop` (`crates/alp-desktop/Cargo.toml`: `[package] name = "alp-desktop"`)
* **Installed Binary Name:** `audio-labs-pro` (The previous release script renamed `alp-desktop` to `audio-labs-pro` during packaging staging).
* **Helper Binaries:** `alp-cli`, `alp-infer`
* **Tauri `productName`:** `"Audio Labs Pro"`
* **Desktop File:** `packaging/desktop/io.github.zvirox.AudioLabsPro.desktop` (standardize to `packaging/audio-labs-pro.desktop`)
* **Icon:** Standardize to `audio-labs-pro.png`
* **Mismatch Assessment:**
  - Rule 1 requires binary name to match the repo name `audio-labs-pro`.
  - In `crates/alp-desktop/Cargo.toml`, we can set `[[bin]] name = "audio-labs-pro" path = "src/main.rs"` or rename the crate, eliminating the manual rename step.

---

## 6. Runtime Dependencies & Extra Files

* **Binaries:**
  * `/usr/bin/audio-labs-pro` (GUI desktop shell)
  * `/usr/bin/alp-cli` (Command-line tool)
  * `/usr/bin/alp-infer` (Offline voice synthesis helper)
* **Desktop File:** `/usr/share/applications/audio-labs-pro.desktop`
* **Mime Type:** `/usr/share/mime/packages/audio-labs-pro.xml`
* **Runtime Dependencies:**
  * Debian/Ubuntu: `libwebkit2gtk-4.1-0`, `libgtk-3-0`, `libasound2`, `libpipewire-0.3-0`
  * Fedora/RPM: `webkit2gtk4.1`, `gtk3`, `alsa-lib`, `pipewire-libs`
  * Arch Linux: `webkit2gtk-4.1`, `gtk3`, `alsa-lib`, `pipewire`

---

## 7. Secrets Used

* **`GH_TOKEN` / `GITHUB_TOKEN`**: Standard repository actions token.

---

## 8. Packaging Formats & Migration Action Table

| Format | Present | Job / File Location | Action |
|---|---|---|---|
| **Debian (.deb)** | Yes | `.github/workflows/release.yml` (`dpkg-deb`) | **Replace** with unified `nfpm` workflow |
| **RPM (.rpm)** | Yes | `.github/workflows/release.yml` (`rpmbuild`) | **Replace** with unified `nfpm` workflow |
| **Arch (.pkg.tar.zst)** | No | None | **Add** via unified `nfpm` workflow |
| **Flatpak** | Yes | `.github/workflows/release.yml` (`flatpak` job) | **Delete** job & `packaging/flatpak/` |
| **AppImage** | No | None | **N/A** |
| **Checksums** | Yes | `sha256sums.txt` | **Replace** with standardized `SHA256SUMS` + signature |
