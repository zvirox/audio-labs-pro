#!/usr/bin/env bash
set -euo pipefail

# Build and assemble Audio Labs Pro AppDir and AppImage bundle
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
BUILD_DIR="${ROOT_DIR}/target/appimage"
APP_DIR="${BUILD_DIR}/AppDir"

echo "== [1/4] Building release binaries =="
cargo build --release --workspace

echo "== [2/4] Assembling AppDir structure =="
rm -rf "${APP_DIR}"
mkdir -p "${APP_DIR}/usr/bin" \
         "${APP_DIR}/usr/share/applications" \
         "${APP_DIR}/usr/share/metainfo" \
         "${APP_DIR}/usr/share/mime/packages" \
         "${APP_DIR}/usr/share/icons/hicolor/scalable/apps"

# Copy binaries
cp "${ROOT_DIR}/target/release/alp-ui" "${APP_DIR}/usr/bin/audio-labs-pro"
cp "${ROOT_DIR}/target/release/alp-cli" "${APP_DIR}/usr/bin/alp-cli"
cp "${ROOT_DIR}/target/release/alp-infer" "${APP_DIR}/usr/bin/alp-infer"
chmod +x "${APP_DIR}/usr/bin/"*

# Copy desktop and metadata
cp "${ROOT_DIR}/packaging/desktop/io.github.zvirox.AudioLabsPro.desktop" "${APP_DIR}/usr/share/applications/"
cp "${ROOT_DIR}/packaging/desktop/io.github.zvirox.AudioLabsPro.desktop" "${APP_DIR}/"
cp "${ROOT_DIR}/packaging/desktop/io.github.zvirox.AudioLabsPro.metainfo.xml" "${APP_DIR}/usr/share/metainfo/"
cp "${ROOT_DIR}/packaging/mime/io.github.zvirox.AudioLabsPro.xml" "${APP_DIR}/usr/share/mime/packages/"
cp "${ROOT_DIR}/packaging/icons/hicolor/scalable/apps/io.github.zvirox.AudioLabsPro.svg" "${APP_DIR}/usr/share/icons/hicolor/scalable/apps/"
cp "${ROOT_DIR}/packaging/icons/hicolor/scalable/apps/io.github.zvirox.AudioLabsPro.svg" "${APP_DIR}/io.github.zvirox.AudioLabsPro.svg"
cp "${ROOT_DIR}/packaging/icons/hicolor/scalable/apps/io.github.zvirox.AudioLabsPro.svg" "${APP_DIR}/.DirIcon"

# Generate AppRun launcher
cat > "${APP_DIR}/AppRun" << 'EOF'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
export PATH="${HERE}/usr/bin:${PATH}"
export LD_LIBRARY_PATH="${HERE}/usr/lib:${HERE}/usr/lib/x86_64-linux-gnu:${LD_LIBRARY_PATH:-}"
export XDG_DATA_DIRS="${HERE}/usr/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
exec "${HERE}/usr/bin/audio-labs-pro" "$@"
EOF
chmod +x "${APP_DIR}/AppRun"

echo "== [3/4] Validating AppDir integrity =="
test -x "${APP_DIR}/AppRun"
test -x "${APP_DIR}/usr/bin/audio-labs-pro"
test -x "${APP_DIR}/usr/bin/alp-cli"
test -x "${APP_DIR}/usr/bin/alp-infer"
test -f "${APP_DIR}/io.github.zvirox.AudioLabsPro.desktop"
test -f "${APP_DIR}/io.github.zvirox.AudioLabsPro.svg"
test -f "${APP_DIR}/.DirIcon"

echo "== [4/4] Generating AppImage bundle =="
OUTPUT_APPIMAGE="${BUILD_DIR}/Audio-Labs-Pro-x86_64.AppImage"
if command -v appimagetool >/dev/null 2>&1; then
    ARCH=x86_64 appimagetool "${APP_DIR}" "${OUTPUT_APPIMAGE}"
    echo "AppImage created successfully at: ${OUTPUT_APPIMAGE}"
else
    echo "appimagetool not found in PATH; AppDir assembled and validated at: ${APP_DIR}"
    echo "To package into a single .AppImage binary, download appimagetool and run: appimagetool ${APP_DIR} ${OUTPUT_APPIMAGE}"
fi

echo "AppDir build complete."
