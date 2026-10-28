# `alp-plugin` / `clap`

## Purpose

Implement safe Rust bindings and C ABI definitions for the open CLAP (CLever Audio Plugin)
standard.

## Public API

- `scan_clap_file`: Inspects a `.clap` dynamic library and extracts plugin descriptors.
- `ActiveClapInstance`: Manages loaded CLAP instances, activation, block rendering, and cleanup.
- CLAP C ABI structs: `ClapHost`, `ClapPlugin`, `ClapProcess`, `ClapAudioBuffer`, `ClapPluginEntry`, `ClapPluginFactory`.

## Data flow

Loads `.clap` shared libraries via `libloading`. Interacts through `clap_entry` to obtain the
plugin factory, instantiates `ClapPlugin`, passes audio buffers via `ClapAudioBuffer`, and
executes `process()`.

## Allowed dependencies

- `libloading`: Dynamic library loading.
- `std::ffi`: C string and raw pointer conversions.
- Parent module types and `PluginError`.

## Threading / real-time notes

`process_block` runs on the real-time audio thread. It uses pre-allocated internal scratch
buffers and performs zero heap allocation.

## Error handling

Returns `PluginError::LibraryLoad`, `MissingSymbol`, `Instantiation`, or `Processing` on
failure.

## Testing

Tested via unit tests and virtual plugin fallback execution.

## Known limits

- Only basic audio processing and parameter extensions are currently hooked; GUI window
  extensions are deferred to future revisions.
