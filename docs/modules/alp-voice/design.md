# `alp-voice` / `design`

## Purpose

Provides voice design synthesis, mapping natural language vocal briefs (e.g. "warm, deep BBC documentary narrator") into customized voice profiles with appropriate pitch, speed, and expressive parameters.

## Public API

- `VoiceDesignRequest`: `name`, `prompt`, `tags`
- `design_voice(request: &VoiceDesignRequest) -> VoiceProfile`

## Data flow

1. Parses natural language descriptors for vocal pitch, speed, energy, and emotional character.
2. Synthesizes a new `VoiceProfile` with `VoiceKind::Designed` and tailored default generation parameters.

## Allowed dependencies

- `alp-voice::{VoiceKind, VoiceParams, VoiceProfile}`
- `serde`, `uuid::Uuid`, Standard library

## Threading / real-time notes

Runs on background workers, UI threads, or out-of-process in `alp-infer`.

## Error handling

Validates input fields and produces unique deterministic profile identifiers.

## Testing

Unit tests in `design.rs` test that natural language keywords correctly modulate pitch, speed, and emotion fields.

## Known limits

- Rule and keyword mapping based for baseline; deep neural voice prompting (VoxCPM2) is delegated through `alp-infer`.
