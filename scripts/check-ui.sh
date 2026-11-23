#!/usr/bin/env bash
# Build-plan rule: Web/Tauri UI uses frontend/src/kit components, zero legacy iced/alp-ui references.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

errors=0

# 1. Verify legacy iced crates are removed
if [[ -d "crates/alp-ui" ]]; then
  echo "FAIL crates/alp-ui directory still exists (legacy iced UI must be removed in M8)"
  errors=$((errors + 1))
fi

if [[ -d "crates/alp-ui-kit" ]]; then
  echo "FAIL crates/alp-ui-kit directory still exists (legacy iced UI kit must be removed in M8)"
  errors=$((errors + 1))
fi

# 2. Verify no legacy iced dependencies in Cargo.toml
iced_deps="$(grep -rnE '^\s*(iced|iced-cube|lilt|alp-ui|alp-ui-kit)\s*=' Cargo.toml crates/*/Cargo.toml 2>/dev/null || true)"
if [[ -n "$iced_deps" ]]; then
  echo "FAIL found legacy iced dependencies in Cargo.toml:"
  echo "$iced_deps"
  errors=$((errors + 1))
fi

# 3. Verify frontend kit exists and contains required core components
required_kit_components=(
  "frontend/src/kit/Button.tsx"
  "frontend/src/kit/Slider.tsx"
  "frontend/src/kit/Toggle.tsx"
  "frontend/src/kit/Input.tsx"
  "frontend/src/kit/Dialog.tsx"
  "frontend/src/kit/Tabs.tsx"
  "frontend/src/kit/Meter.tsx"
  "frontend/src/kit/index.ts"
)

for component in "${required_kit_components[@]}"; do
  if [[ ! -f "$component" ]]; then
    echo "FAIL missing required frontend kit component: $component"
    errors=$((errors + 1))
  fi
done

if [[ $errors -gt 0 ]]; then
  echo "check-ui: FAILED with $errors error(s)"
  exit 1
fi

echo "check-ui: OK (frontend kit verified, zero legacy iced references)"
