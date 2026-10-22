#!/usr/bin/env bash
# Build-plan rule: screens use alp-ui-kit controls, never raw iced buttons.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

hits="$(
  grep -RInE \
    'iced::widget::button|iced::widget::\{[^}]*\bbutton\b|use[[:space:]]+iced::widget::button' \
    crates/alp-ui/src --include='*.rs' || true
)"

if [[ -n "$hits" ]]; then
  echo "FAIL raw iced button in alp-ui (use alp-ui-kit::components instead):"
  echo "$hits"
  exit 1
fi

echo "check-ui: OK (no raw iced::widget::button in alp-ui)"
