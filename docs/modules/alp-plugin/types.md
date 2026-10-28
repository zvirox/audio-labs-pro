# `alp-plugin` / `types`

## Purpose

Define core metadata models, descriptors, format enums, parameter structures, and
processing execution contexts for audio plugin hosting.

## Public API

- `PluginFormat`: Supported plugin standards (`Clap`, `Lv2`).
- `PluginCategory`: Functional categorizations (`Effect`, `Instrument`, `Restoration`, `Analyzer`, `Utility`).
- `PluginDescriptor`: Serializable bundle and capability metadata.
- `PluginParameter`: Automatable parameter descriptors with normalized clamping helpers.
- `ProcessContext`: Real-time block context (`sample_rate`, `num_frames`, `frame_position`, `is_playing`).

## Data flow

Generated during scanning and instance creation. Serialized into project files and JSON
caches. Passed to audio processing callbacks.

## Allowed dependencies

- `serde`, `serde_json`: Serialization and deserialization.
- `std::path::PathBuf`.

## Threading / real-time notes

`ProcessContext` is a `Copy` struct passed by reference into audio processing loops with
zero allocation overhead.

## Error handling

Parameter values are clamped within valid ranges (`min_value..=max_value`).

## Testing

Unit tests for parameter normalization, clamping, and fraction conversions.

## Known limits

- Only stereo bus configurations are currently modeled in `PluginDescriptor`.
