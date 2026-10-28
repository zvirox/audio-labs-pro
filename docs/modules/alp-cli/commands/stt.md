# `alp-cli` / `commands` / `stt`

## Purpose

Provides CLI commands for speech-to-text transcription (`alp-cli transcribe`) and voice activity detection (`alp-cli vad`), supporting multiple subtitle formats (TXT, SRT, VTT) and machine-readable JSON output.

## Public API

- `transcribe(input: &Path, output: Option<&Path>, format: &str, json: bool) -> Result<()>`
- `vad(input: &Path, threshold_db: f32, hangover_ms: u32, min_speech_ms: u32, json: bool) -> Result<()>`

## Data flow

1. `transcribe` decodes input media, sends audio to `alp-infer` daemon (or falls back to in-process STT), formats results as TXT, SRT, VTT, or JSON, and writes to disk or stdout.
2. `vad` decodes input media, executes speech segmentation with configured dBFS threshold and hangover durations, and outputs formatted timestamped intervals and total speech percentages.

## Allowed dependencies

- `alp-media::decode`
- `alp-voice::{BuiltinSttBackend, InferClient, SttBackend, Transcript, VadConfig, segment_speech}`
- `anyhow`, `serde_json`, Standard library

## Threading / real-time notes

CLI command execution runs synchronously on the main thread and terminates upon command completion.

## Error handling

Returns `anyhow::Result` with contextual error messages if audio decoding, IPC communication, or file writing encounters errors.

## Testing

Covered by CLI integration tests and the automated Phase 7 demo acceptance harness `scripts/phase7-demo.sh`.

## Known limits

- Transcription quality depends on the active model; defaults to built-in acoustic STT if neural models are not downloaded.
