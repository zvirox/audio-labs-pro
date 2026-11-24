# `alp-mcp` / `starship`

## Purpose

Starship Control plugin bridge for Audio Labs Pro.
Exposes voice operations, project inspection, cleanup workflows, and checkpoint management over `$XDG_RUNTIME_DIR/starship/bridges/alp.sock`, and publishes the discovery manifest at `$XDG_DATA_HOME/starship/plugins/alp.json`.

## Public API

- `build_alp_bridge() -> PluginBridge`

## Data flow

1. Builds an `AppManifest` identifying Audio Labs Pro as `alp`.
2. Registers curated tools for voice synthesis, track inspection, effect application, and project checkpoints.
3. Evaluates requests and provides response payloads over the local Unix domain socket bridge.

## Allowed dependencies

- `starship_plugin::{AppManifest, PluginBridge, PluginTool, bridge_dir}`
- `serde_json::{json, Value}`
- `std::sync::{Arc, Mutex}`

## Threading / real-time notes

The bridge handles requests out-of-process from the real-time audio thread. Mutex synchronization protects shared checkpoint state without blocking audio playback.

## Error handling

Missing checkpoints return JSON-RPC tool error results. Malformed tool invocations report schema validation errors back to Starship Control.

## Testing

Unit tests in `crates/alp-mcp/src/starship.rs` verify tool registration, tool calls, and checkpoint create/restore roundtrips.

## Known limits

- In-memory checkpoints only; checkpoints are preserved during application runtime but reset upon application exit.
