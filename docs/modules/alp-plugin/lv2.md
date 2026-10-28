# `alp-plugin` / `lv2`

## Purpose

Implement safe Rust bindings, bundle discovery, and C ABI host execution for the LV2
Linux Audio standard.

## Public API

- `scan_lv2_bundle`: Scans an LV2 bundle directory, parses `manifest.ttl`, and locates shared objects.
- `ActiveLv2Instance`: Manages instantiated LV2 plugins, port connections, block execution, and cleanup.
- LV2 C ABI structs: `Lv2Descriptor`, `Lv2Feature`, `Lv2Handle`.

## Data flow

Scans `/usr/lib/lv2` and custom directories for `.lv2` folders. Dynamically links `.so` binaries,
calls `lv2_descriptor(0)`, invokes `instantiate()`, maps stereo input/output audio ports via
`connect_port()`, and executes `run()`.

## Allowed dependencies

- `libloading`: Dynamic library loading.
- `std::fs`, `std::ffi`, `std::path`.
- Parent module types and `PluginError`.

## Threading / real-time notes

`process_block` runs in the hard real-time audio loop. Audio ports are pre-connected to
fixed scratch memory buffers without allocations or mutex locks.

## Error handling

Manifest parsing and dynamic symbol errors are converted to descriptive `PluginError` variants.

## Testing

Tested against system LV2 plugins (`/usr/lib/lv2`) and mock instances.

## Known limits

- Only audio and control ports are processed; MIDI and custom atom event ports are ignored.
