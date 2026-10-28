# `alp-plugin` / `scanner`

## Purpose

Traverse filesystem directory hierarchies across standard Linux audio search paths
to discover, inspect, and cache CLAP and LV2 plugins.

## Public API

- `PluginScanner`: Scanner engine managing paths and persistent caching.
  - `new`: Sets up standard Linux paths (`/usr/lib/clap`, `/usr/lib/lv2`, etc.).
  - `add_search_path`: Appends custom directory.
  - `scan`: Traverses paths and produces deduplicated `PluginDescriptor` lists.
  - `load_cache`: Loads cached descriptors from `$XDG_CACHE_HOME/alp/plugins.json`.

## Data flow

Discovers `.clap` files and `.lv2` bundles. Invokes format-specific inspectors and writes
a JSON cache to avoid scanning overhead on application launch.

## Allowed dependencies

- `std::env`, `std::fs`, `std::path`.
- `tracing`: Diagnostic logging.
- `serde`, `serde_json`: Cache serialization.
- `PluginError`, `PluginDescriptor`.

## Threading / real-time notes

Scanning is an I/O heavy operation intended strictly for startup or background worker
tasks, never in the audio thread.

## Error handling

Unreadable files or corrupt bundles log warnings and are skipped without terminating the
entire scan.

## Testing

Unit tests for custom directory scanning, cache persistence, and descriptor deduplication.

## Known limits

- Does not follow recursive symlink cycles across filesystem boundaries.
