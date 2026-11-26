# `alp-plugin` / `vst3`

## Purpose

VST3 bundle discovery for Linux. Scans standard `.vst3` bundle locations and records their shared-library payloads as `PluginDescriptor`s so the plugin inventory is complete.

## Public API

- `scan_vst3_bundle(path) -> Result<PluginDescriptor, PluginError>`.
- `instantiation_error(desc) -> PluginError`.
- `PluginFormat::Vst3` with display name `VST3` and extension `vst3`.

## Data flow

1. Locate a `.vst3` bundle directory under `/usr/lib/vst3`, `/usr/local/lib/vst3`, `$HOME/.vst3`, or `VST3_PATH`.
2. Search up to four directory levels for a `.so` payload.
3. Return a descriptor with id `vst3:<bundle name>`, stereo I/O, and `param_count = 0`.
4. `ActivePlugin::instantiate` returns `PluginError::Instantiation` for VST3 descriptors — no fake host.

## Allowed dependencies

`alp-core` (via plugin types) and `serde`. No external VST3 SDK; all code is safe Rust filesystem traversal inside a crate that already permits unsafe for CLAP/LV2.

## Threading / real-time notes

Discovery is filesystem I/O and runs off the audio thread during `PluginHost::rescan`. No real-time path is affected because VST3 descriptors cannot be instantiated.

## Error handling

Non-directory paths and bundles without a shared library return `PluginError::Discovery` with the offending path. Scanner logs discovery failures and continues with other bundles.

## Testing

Tests verify rejection of non-directories, descriptor fields for a synthetic bundle, and that instantiation produces an explicit "not implemented" error.

## Known limits

- **No VST3 C++ COM ABI host is implemented in this build.** Discovered plugins cannot be loaded or processed; the failure is explicit and never silently substituted.
- No vendor/version metadata is parsed from the bundle.
- Native editor GUI parenting is therefore not exercised by VST3 yet.
