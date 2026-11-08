# `alp-api` / `effect_ops`

## Purpose

Exposes DSP effect catalog queries, effect insertions, updates, removals, and preset applications across project tracks.

## Public API

- `EffectDescriptor`: effect ID, human-readable name, category, JSON Schema parameters.
- `PresetDescriptor`: preset ID, name, description.
- `EffectCatalogReport`: list of available effects and composite chain presets.
- `ApiSession::effect_catalog(&self) -> Result<EffectCatalogReport, ApiError>`
- `ApiSession::effect_apply(&mut self, track_id: &str, effect_id: &str, params: serde_json::Value) -> Result<(), ApiError>`
- `ApiSession::effect_update(&mut self, track_id: &str, effect_index: usize, params: serde_json::Value) -> Result<(), ApiError>`
- `ApiSession::effect_remove(&mut self, track_id: &str, effect_index: usize) -> Result<(), ApiError>`
- `ApiSession::preset_apply(&mut self, track_id: &str, preset_id: &str) -> Result<(), ApiError>`

## Data flow

1. Verifies permission tier (`PermissionTier::Observe` for catalog, `PermissionTier::Edit` for mutations).
2. For mutations, locates track by ID and clones current effect list.
3. Modifies target effect or constructs pre-configured chains (e.g. `voice_cleanup` or `podcast_master`).
4. Dispatches an undoable `SetTrackEffects` command onto the session command stack.

## Allowed dependencies

- `serde::{Deserialize, Serialize}`
- `alp_core::{EffectRef, SetTrackEffects}`
- `crate::error::ApiError`
- `crate::session::ApiSession`
- `crate::types::PermissionTier`

## Threading / real-time notes

Mutations execute on worker/session threads under session lock; changes propagate to the real-time mixer on the next audio frame.

## Error handling

Returns `ApiError::NotFound` if the track ID or preset ID does not exist; `ApiError::Validation` if effect indices are out of range.

## Testing

Covered by effect mutation and catalog integration tests in `alp-api` and `alp-mcp`.

## Known limits

- Effect updates currently replace the entire parameter payload rather than merging incremental parameter diffs.
