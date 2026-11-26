#!/usr/bin/env bash
# Phase 19 release-readiness acceptance: packaging config parity, local nfpm
# dry-run for deb/rpm/archlinux, and SHA256SUMS generation/verification.
#
# This does NOT tag, push, sign, or publish anything. Use --build to compile
# the release binaries locally before the dry-run.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

BUILD=0
OUTPUT_DIR="${OUTPUT_DIR:-/tmp/opencode/alp-phase19-packages}"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --build)
      BUILD=1
      shift
      ;;
    --outdir)
      OUTPUT_DIR="${2:?--outdir requires a value}"
      shift 2
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

echo "== [1/5] Repository guards =="
bash scripts/check-file-length.sh
bash scripts/check-deps.sh
bash scripts/check-docs.sh
bash scripts/check-ui.sh

echo "== [2/5] Version and packaging config parity =="
VERSION="$(sed -nE 's/^version = "(.*)"/\1/p' Cargo.toml | head -n 1)"
if [[ -z "${VERSION}" ]]; then
  echo "Could not read workspace version from Cargo.toml" >&2
  exit 1
fi
echo "  workspace version: ${VERSION}"

grep -q 'src: target/release/alp-mcp' packaging/nfpm.yaml || {
  echo "nfpm.yaml does not install alp-mcp" >&2
  exit 1
}
grep -q -- '-p alp-mcp' .github/workflows/release.yml || {
  echo "release.yml does not build alp-mcp" >&2
  exit 1
}
mkdir -p "${OUTPUT_DIR}"

if (( BUILD )); then
  echo "== [3/5] Building release binaries =="
  cargo build --release --locked -p alp-desktop -p alp-cli -p alp-infer -p alp-mcp
else
  echo "== [3/5] Skipping release build (pass --build to compile) =="
fi

echo "== [4/5] Local nfpm package dry-run =="
missing=0
while IFS= read -r src; do
  if [[ ! -e "${src}" ]]; then
    echo "  missing packaging asset: ${src}" >&2
    missing=1
  fi
done < <(grep -E '^[[:space:]]*- src:[[:space:]]*' packaging/nfpm.yaml | sed -E 's/^[[:space:]]*- src:[[:space:]]*//')
if (( missing )); then
  echo "Release binaries/assets are missing; re-run with --build" >&2
  exit 1
fi

if ! command -v nfpm >/dev/null 2>&1; then
  echo "nfpm is not installed; cannot run the local dry-run" >&2
  exit 1
fi
rm -f "${OUTPUT_DIR}"/*.deb "${OUTPUT_DIR}"/*.rpm "${OUTPUT_DIR}"/*.pkg.tar.zst "${OUTPUT_DIR}"/SHA256SUMS
for packager in deb rpm archlinux; do
  echo "  packaging ${packager}"
  VERSION="${VERSION}" nfpm package -f packaging/nfpm.yaml -p "${packager}" -t "${OUTPUT_DIR}"
done

if command -v dpkg-deb >/dev/null 2>&1; then
  deb_package="$(find "${OUTPUT_DIR}" -name '*.deb' -print -quit)"
  if ! dpkg-deb --contents "${deb_package}" | grep -q '/usr/bin/alp-mcp'; then
    echo "deb package does not contain /usr/bin/alp-mcp" >&2
    exit 1
  fi
  echo "  verified: deb ships /usr/bin/alp-mcp"
fi

echo "== [5/5] SHA256SUMS generation and verification =="
(
  cd "${OUTPUT_DIR}"
  sha256sum ./*.deb ./*.rpm ./*.pkg.tar.zst > SHA256SUMS
  sha256sum -c SHA256SUMS
)
echo "Artifacts:"
ls -lh "${OUTPUT_DIR}"
echo "Phase 19 release-readiness checks passed (no tag, push or signature created)"
