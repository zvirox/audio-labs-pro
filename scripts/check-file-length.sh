#!/usr/bin/env bash
# Build-plan rule: no source file over 1,500 lines.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

limit=1500
warn=1000
status=0

while IFS= read -r -d '' file; do
  if head -n 3 "$file" | grep -q '@generated'; then
    continue
  fi
  lines=$(wc -l <"$file")
  if (( lines > limit )); then
    printf 'FAIL %5d lines (limit %d): %s\n' "$lines" "$limit" "$file"
    status=1
  elif (( lines > warn )); then
    printf 'WARN %5d lines (soft %d): %s\n' "$lines" "$warn" "$file"
  fi
done < <(find crates -type f -name '*.rs' -print0 | sort -z)

if (( status == 0 )); then
  echo "check-file-length: OK (no Rust file over ${limit} lines)"
fi
exit "$status"
