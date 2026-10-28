# `alp-cli` / `commands` / `voice`

## Purpose

Provides CLI commands for text-to-speech synthesis (`tts`), model catalogue inspection, checksum verification, license acceptance (`model`), and script-mode multi-line batch generation (`script`).

## Public API

- `tts(text: &str, output: &Path, voice: &str, speed: f32) -> Result<()>`
- `model_list(json: bool) -> Result<()>`
- `model_verify(model_str: &str) -> Result<()>`
- `model_accept_license(model_str: &str) -> Result<()>`
- `script_render(script_path: &Path, output: &Path, stitch: bool) -> Result<()>`

## Data flow

1. `tts` sends synthesis requests to `alp-infer` via `InferClient` or falls back to in-process `BuiltinAcousticBackend`.
2. Encodes the resulting PCM buffer to 24-bit WAV using `alp_media::export`.
3. `model_*` queries `alp_voice::ModelManager` to inspect, verify SHA-256 sums, or accept license agreements.
4. `script_render` reads a multi-line JSON script, synthesizes each line independently, and either stitches with pause gaps or exports stem files.

## Allowed dependencies

- `anyhow`
- `alp-core`, `alp-media`, `alp-voice`
- Standard library

## Threading / real-time notes

Runs on the CLI command thread. Speech synthesis is CPU-bound; I/O occurs via standard blocking filesystem calls.

## Error handling

Returns `anyhow::Result` with detailed error context on I/O failures, model verification mismatches, or synthesis errors.

## Testing

Acceptance tested in `scripts/phase6-demo.sh` verifying end-to-end speech generation, model listing, and script rendering.

## Known limits

- Script rendering operates sequentially per line; parallel line batching will be added in future optimizations.
