# `alp-engine` / `insert_rack`

## Purpose

Per-track insert rack for live playback and offline render. A rack holds up to eight DSP inserts built from a project's `EffectRef` chain. A shared `InsertRackControl` handle exposes relaxed atomics so bypass and parameters can be changed from the UI without locking or touching the audio thread.

## Public API

- `MAX_INSERT_SLOTS = 8`, `INSERT_PARAM_COUNT = 8`.
- `InsertRackControl`: `set_enabled(slot, bool)`, `set_param(slot, param, f32)`, `is_enabled(slot)`, `param(slot, param)`, cloneable.
- `InsertSlotInfo`: `index`, `id`, `kind`, `param_names`, `enabled` for IPC/UI display.
- `InsertRack`: `from_effects(effects, sample_rate, channels)`, `empty()`, `control()`, `slot_info(id_prefix)`, `process(&mut [f32])`, `reset()`.

## Data flow

1. `MixScene::from_tracks` copies each track's `EffectRef` chain into the scene as an insert descriptor list.
2. `Mixer::with_master` builds one `InsertRack` per track and keeps its control handle for the session.
3. `Mixer::render_into` sums clips into a per-track scratch buffer, runs the rack, then sums tracks into the master bus.
4. The UI reads `slot_info` and writes parameters through `InsertRackControl`; no IPC message is drained inside the callback.

## Allowed dependencies

`alp-core` (`EffectRef`, `TrackId`) and `alp-dsp` (concrete DSP effects). The crate already depends on `alp-media` and `alp-plugin` for decoding and hosted plugins. No UI, session, or inference dependencies.

## Threading / real-time notes

`InsertRack::process` reads relaxed atomics and calls prebuilt `Effect` instances. It allocates nothing, takes no locks, logs nothing, and touches no files. Slot configuration is fixed at rack construction; changing the effect chain requires rebuilding the scene/Mixer off the audio thread.

## Error handling

Unknown effect kinds are logged once at construction (outside the audio thread) and replaced with unity gain. Missing or non-finite parameters fall back to per-effect defaults. Control writes to out-of-range slots or parameters are silently ignored.

## Testing

Tests in `crates/alp-engine/src/insert_rack.rs` cover slot truncation at eight, exact bypass for disabled slots, live parameter changes visible to processing, and unity-gain fallback for unknown kinds. Mixer tests verify that insert processing does not change output when racks are empty.

## Known limits

- Bypass/parameter changes are live; adding, removing, or reordering effects requires rebuilding the rack.
- Only the first three EQ parameters and the new repair effects have live setters; other DSP effects use their construction parameters until rebuilt.
- Per-track scratch buffers are reserved for 4096 frames; unusually large blocks may reallocate once.
