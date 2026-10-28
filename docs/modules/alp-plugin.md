# `alp-plugin`

## Purpose

CLAP and LV2 audio plugin hosting engine for Audio Labs Pro. Provides standard
filesystem discovery, dynamic library loading, unified real-time audio block
processing, parameter automation, and state management.

## Public API

- `PluginHost`: Central plugin discovery and lifecycle manager.
- `TrackPluginChain`: Serial real-time execution chain for track effects.
- `ActivePlugin`: Live plugin instance abstraction for block DSP and parameters.
- `PluginScanner`: Path scanner for CLAP and LV2 bundles.
- `PluginDescriptor`, `PluginParameter`, `PluginFormat`, `PluginCategory`: Metadata and automation types.
- `PluginError`: Structured error reporting.

## Data flow

Discovers installed plugins in standard Linux audio paths (`/usr/lib/clap`, `/usr/lib/lv2`,
`~/.clap`, `~/.lv2`). Instantiates active plugins via C ABI bridges, connects audio scratch
buffers, and processes blocks in series on track effect chains.

## Allowed dependencies

- `alp-core`: Core types and project structures.
- `libloading`: Dynamic shared library loading.
- `serde`, `serde_json`: Serialization and caching.
- `thiserror`, `tracing`, `uuid`.

## Threading / real-time notes

Discovery and dynamic library loading occur on background or worker threads. Once
instantiated, `process()` executes on the real-time audio thread without heap allocations
or blocking mutex locks.

## Error handling

Returns typed `PluginError` on failed loading, missing entrypoint symbols, instantiation
failures, or invalid parameter IDs.

## Testing

Unit tests for parameter normalization, stub DSP processing, path scanning, caching,
and multi-plugin serial chaining. Verified via `scripts/phase11-demo.sh`.

## Known limits

- In-process execution assumes well-behaved third-party plugins that do not crash the host process.
- Multi-channel surround plugins are currently down-mixed or clamped to stereo.
