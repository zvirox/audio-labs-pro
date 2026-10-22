#!/usr/bin/env bash
# Build-plan rule: every module has a document with the required sections.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

python3 - <<'PY'
from pathlib import Path
import sys

ROOT = Path(".")
CRATES = ROOT / "crates"
DOCS = ROOT / "docs" / "modules"
SECTIONS = [
    "Purpose",
    "Public API",
    "Data flow",
    "Allowed dependencies",
    "Threading / real-time notes",
    "Error handling",
    "Testing",
    "Known limits",
]

errors = []
checked = 0


def check_doc(path: Path, origin: Path) -> None:
    global checked
    if not path.is_file():
        errors.append(f"missing {path} for {origin}")
        return
    text = path.read_text(encoding="utf-8")
    headings = {line[3:].strip() for line in text.splitlines() if line.startswith("## ")}
    missing = [section for section in SECTIONS if section not in headings]
    if missing:
        errors.append(f"{path} is missing sections: {', '.join(missing)}")
    checked += 1


for crate in sorted(path for path in CRATES.iterdir() if path.is_dir()):
    if not (crate / "Cargo.toml").is_file():
        continue
    name = crate.name
    check_doc(DOCS / f"{name}.md", crate / "Cargo.toml")
    src = crate / "src"
    if not src.is_dir():
        continue
    for source in sorted(src.rglob("*.rs")):
        relative = source.relative_to(src)
        if relative.name in {"lib.rs", "main.rs"}:
            continue
        if relative.name == "mod.rs":
            stem = relative.parent
        else:
            stem = relative.with_suffix("")
        doc = DOCS / name / f"{stem}.md"
        check_doc(doc, source)

if errors:
    for error in errors:
        print(f"FAIL {error}")
    sys.exit(1)

print(f"check-docs: OK ({checked} documents, all required sections present)")
PY
