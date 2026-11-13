# `alp-api` / `effect_ops`

## Purpose

Serves the effect/preset catalog and mutates track effect chains. Effect ids and
preset names are validated against the catalog, so a typo fails with the known
list instead of inserting an inert chain entry.

## Public API

- `EffectDescriptor` — `id`, `name`, `category`, parameter schema.
- `PresetDescriptor` — `id`, `name`, `description`.
- `EffectCatalogReport` — `effects: Vec<EffectDescriptor>`, `presets:
  Vec<PresetDescriptor>`.
- `ApiSession::effect_catalog(&self) -> Result<EffectCatalogReport, ApiError>`
- `ApiSession::effect_apply(&mut self, track_id: &str, effect_id: &str, params: serde_json::Value) -> Result<String, ApiError>` — returns the new chain-entry handle.
- `ApiSession::effect_update(&mut self, track_id: &str, effect_id: &str, params: serde_json::Value) -> Result<(), ApiError>`
- `ApiSession::effect_remove(&mut self, track_id: &str, effect_id: &str) -> Result<(), ApiError>`
- `ApiSession::preset_apply(&mut self, track_id: &str, preset: &str) -> Result<(), ApiError>`

Catalog ids include `parametric_eq`, `compressor`, `de_esser`, `gate`,
`de_hum`, `de_click`, `limiter`; presets are `voice_cleanup` and
`podcast_master`.

## Data flow

1. `effect_catalog` requires `Observe`; mutations require `Edit`.
2. `effect_apply` validates `effect_id` against the catalog, clones the track's
   chain, allocates a `fx-<uuid>` handle, copies numeric params into the
   `EffectRef`, appends it, and applies a `SetTrackEffects` command.
3. `effect_update` finds the chain entry by handle and replaces its numeric
   params, then re-applies `SetTrackEffects`.
4. `effect_remove` drops the entry by handle.
5. `preset_apply` validates the preset name, then replaces the whole chain with
   a single `preset:<name>` ref (a label that records the intent).

Validation helpers `validate_effect_id` and `validate_preset` reuse
`effect_catalog` and format the accepted ids into the error message.

## Allowed dependencies

- `serde::{Deserialize, Serialize}`
- `alp_core::{EffectRef, SetTrackEffects, TrackId}`
- `crate::error::ApiError`
- `crate::session::ApiSession`
- `crate::types::PermissionTier`

## Threading / real-time notes

Catalog reads and chain mutations are synchronous on the caller's thread. Chain
changes land in the project; the offline renderer does not yet process them, so
no audio is recomputed here.

## Error handling

- `ApiError::Validation` with the accepted list when `effect_id` or `preset`
  is unknown.
- `ApiError::NotFound` when the track or chain-entry handle does not exist.
- `ApiError::PermissionDenied` below `Edit`.

## Testing

Unit tests in `effect_ops.rs`:

- `a_known_effect_lands_in_the_chain`
- `an_unknown_effect_id_is_rejected_with_the_catalog`
- `an_unknown_preset_is_rejected_and_a_known_one_applies`

Exposed through MCP by `effect_apply`, `effect_update`, `effect_remove`,
`preset_apply`, and the `alp://effects/catalog` resource (which serialises the
live catalog).

## Known limits

- The offline renderer does not apply track effect chains, so inserting or
  changing an effect changes the project, not the rendered audio.
- `preset_apply` records one `preset:<name>` ref rather than expanding the
  preset into its constituent effect chain.
- `effect_update` replaces the numeric parameter set rather than merging a
  partial diff.
