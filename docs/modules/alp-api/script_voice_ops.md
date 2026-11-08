# `alp-api` / `script_voice_ops`

## Purpose

Provides script-based multi-line speech synthesis and single-clip speech regeneration tools for `ApiSession`.

## Public API

- `ScriptRegenerateReport`: clip ID, new transcript text, synthesizing engine name, new audio duration.
- `ApiSession::tts_regenerate(&mut self, clip_id: &str, new_text: &str, voice_id: &str, speed: Option<f32>) -> Result<ScriptRegenerateReport, ApiError>`
- `ApiSession::tts_script_generate(&mut self, script_json: &str) -> Result<(), ApiError>`

## Data flow

1. Verifies caller permissions under `PermissionTier::Edit`.
2. For `tts_regenerate`:
   - Synthesizes audio using target voice profile via `ApiSession::tts_generate`.
   - Records the synthesizing engine name and resulting duration.
   - Updates clip metadata and returns `ScriptRegenerateReport`.
3. For `tts_script_generate`: parses structured script dialogue lines and queues generation across timeline tracks.

## Allowed dependencies

- `serde::{Deserialize, Serialize}`
- `crate::error::ApiError`
- `crate::session::ApiSession`
- `crate::types::PermissionTier`

## Threading / real-time notes

Speech synthesis calls are dispatched to the out-of-process `alp-infer` daemon over Unix domain sockets, keeping the main process responsive.

## Error handling

Returns `ApiError::Validation` if the voice profile is missing or invalid; `ApiError::PermissionDenied` if the session tier is Observe-only.

## Testing

Covered by speech regeneration and script automation tests in `alp-api` and `alp-mcp`.

## Known limits

- Batch script generation currently creates clips sequentially rather than in parallel synthesis batches.
