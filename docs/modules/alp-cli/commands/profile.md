# `alp-cli` / `commands` / `profile`

## Purpose

Provides CLI commands under `alp-cli profile` for listing, inspecting, designing, importing, exporting, and deleting voice profiles, auditing reference audio quality, comparing takes, and probing system hardware.

## Public API

- `list(json: bool) -> Result<()>`
- `info(id: &str, json: bool) -> Result<()>`
- `check_reference(input: &Path, json: bool) -> Result<()>`
- `design(name: &str, prompt: &str, tags: Vec<String>, json: bool) -> Result<()>`
- `export_profile(id: &str, output: &Path) -> Result<()>`
- `import_profile(bundle_path: &Path, json: bool) -> Result<()>`
- `compare(reference: &Path, generated: &Path, json: bool) -> Result<()>`
- `delete(id: &str) -> Result<()>`
- `hardware_probe(json: bool) -> Result<()>`

## Data flow

1. Interacts with `ProfileStore` in `$XDG_DATA_HOME/alp/voices/`.
2. Connects to `alp-infer` over Unix domain socket for design and cloning tasks where available.
3. Formats terminal tables or JSON payloads for scripting and automation.

## Allowed dependencies

- `alp-media::decode`
- `alp-voice::{HardwareProfile, InferClient, ProfileStore, ReferenceQualityReport, VoiceDesignRequest, analyze_reference_quality, compare_audio, design_voice}`
- `anyhow`, `serde_json`, Standard library

## Threading / real-time notes

Synchronous command-line execution terminating upon completion.

## Error handling

Returns detailed `anyhow::Result` error messages on missing profiles, failed quality gates, or invalid ZIP packages.

## Testing

Exercised by workspace tests and verified end-to-end in `scripts/phase8-demo.sh`.

## Known limits

- Interactive consent dialog is human-only in the UI. CLI profile import is automatable and enforces the library's structural consent gate — a cloned profile must carry a well-formed consent record, and kind/consent mismatches are rejected — but records are not cryptographically signed, so structural enforcement is not proof of authenticity.
