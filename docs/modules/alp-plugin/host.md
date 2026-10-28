# `alp-plugin` / `host`

## Purpose

Provide the high-level plugin host interface coordinating descriptor registries,
virtual reference plugins, and real-time serial execution chains.

## Public API

- `PluginHost`: Central coordinator for listing, rescanning, and instantiating plugins.
- `TrackPluginChain`: Ordered serial chain of active plugins attached to an audio track or master bus.
  - `add_plugin`, `remove_plugin`, `plugins`, `plugins_mut`.
  - `process_chain`: Runs all active plugins in series over audio buffers.

## Data flow

UI and automation engines query `PluginHost` to list and instantiate plugins. Instantiated
plugins are appended to `TrackPluginChain` instances and processed in series during track
mix-down.

## Allowed dependencies

- `alp-core`.
- Internal `instance`, `scanner`, and `types` submodules.
- `std::sync::{Arc, Mutex}`.

## Threading / real-time notes

`PluginHost` operations are thread-safe via `Arc<Mutex<...>>`. `TrackPluginChain::process_chain`
runs in real time using preallocated scratch buffers.

## Error handling

Returns `PluginError::NotFound` when querying non-existent plugin IDs.

## Testing

Unit tests for host initialization, virtual plugin instantiation, and serial chain processing.

## Known limits

- Track chains process plugins strictly in series; parallel routing splits are deferred.
