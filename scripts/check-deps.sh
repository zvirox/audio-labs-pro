#!/usr/bin/env bash
# Build-plan rule: crate dependency directions are fixed by the architecture.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

python3 - <<'PY'
import json
import subprocess
import sys

ALLOWED = {
    "alp-core": set(),
    "alp-media": {"alp-core"},
    "alp-dsp": {"alp-core"},
    "alp-plugin": {"alp-core"},
    "alp-engine": {"alp-core", "alp-media", "alp-plugin"},
    "alp-voice": {"alp-core", "alp-media"},
    "alp-infer": {"alp-core", "alp-media", "alp-voice"},
    "alp-ui-kit": set(),
    "alp-api": {"alp-core", "alp-dsp", "alp-engine", "alp-media", "alp-plugin", "alp-voice"},
    "alp-mcp": {"alp-api", "alp-core"},
    "alp-ui": {"alp-api", "alp-core", "alp-engine", "alp-media", "alp-plugin", "alp-ui-kit", "alp-voice"},
    "alp-cli": {"alp-core", "alp-dsp", "alp-engine", "alp-media", "alp-plugin", "alp-voice"},
}

result = subprocess.run(
    ["cargo", "metadata", "--no-deps", "--format-version", "1"],
    capture_output=True,
    text=True,
)
if result.returncode != 0:
    print("FAIL cargo metadata failed:")
    print(result.stderr.strip())
    sys.exit(1)

try:
    metadata = json.loads(result.stdout)
except json.JSONDecodeError as error:
    print(f"FAIL could not parse cargo metadata: {error}")
    sys.exit(1)

packages = {package["name"]: package for package in metadata["packages"]}
errors = []

for expected in ALLOWED:
    if expected not in packages:
        errors.append(f"missing workspace crate {expected}")

for name, package in packages.items():
    if not name.startswith("alp-"):
        continue
    if name not in ALLOWED:
        errors.append(f"{name} is not in the dependency map")
        continue
    actual = {
        dependency["name"].replace("_", "-")
        for dependency in package["dependencies"]
        if dependency["name"].replace("_", "-").startswith("alp-")
    }
    expected = ALLOWED[name]
    if actual != expected:
        missing = sorted(expected - actual)
        extra = sorted(actual - expected)
        if missing:
            errors.append(f"{name} is missing allowed dependencies: {', '.join(missing)}")
        if extra:
            errors.append(f"{name} has forbidden ALP dependencies: {', '.join(extra)}")
    if name != "alp-infer" and "alp-infer" in actual:
        errors.append(f"{name} must not depend on alp-infer (separate process)")

if errors:
    for error in errors:
        print(f"FAIL {error}")
    sys.exit(1)

print(f"check-deps: OK ({len(ALLOWED)} ALP crates, dependency directions match the architecture)")
PY
