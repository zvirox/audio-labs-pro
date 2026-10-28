# `alp-plugin` / `instance`

## Purpose

Provide a unified, format-agnostic `ActivePlugin` instance interface for parameter
manipulation, real-time audio block execution, bypass control, and state persistence.

## Public API

- `ActivePlugin`: Instantiated audio plugin managing parameter maps, bypass state, and audio rendering.
  - `instantiate`: Creates and activates an instance from a descriptor.
  - `process`: Real-time block rendering.
  - `set_parameter`, `get_parameter`, `parameters`: Automation and inspection.
  - `save_state`, `restore_state`: JSON/base64 state serialization.

## Data flow

Wraps `ActiveClapInstance`, `ActiveLv2Instance`, or virtual fallback instances. Receives
audio blocks from track mixing loops, delegates to the underlying C ABI, and returns
rendered samples.

## Allowed dependencies

- `alp-core`: Types and errors.
- Internal `clap` and `lv2` submodules.
- `serde`, `serde_json`: State serialization.

## Threading / real-time notes

`process` is guaranteed lock-free and allocation-free. Parameter updates are stored in
memory and applied synchronously.

## Error handling

Returns `PluginError::UnknownParameter` on invalid symbols, and `PluginError::State` on
malformed state strings.

## Testing

Unit tests for stub processing, gain adjustments, bypass behavior, and state save/restore.

## Known limits

- Highly complex opaque chunk states with external file dependencies are not yet supported.
