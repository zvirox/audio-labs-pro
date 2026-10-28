# `alp-plugin` / `error`

## Purpose

Define the typed error model for audio plugin discovery, dynamic loading, ABI validation,
and real-time execution.

## Public API

- `PluginError`: Enum covering discovery, library load, ABI mismatch, missing symbol,
  instantiation, activation, processing, parameter, state, core, and I/O failures.

## Data flow

Constructed in scanner, CLAP host, LV2 host, and active instance methods when external
libraries or operations fail.

## Allowed dependencies

- `thiserror`: Derive macro for error display formatting.
- `alp-core::CoreError`: Core error conversion.

## Threading / real-time notes

Error types allocate strings only on failure paths outside the real-time processing loop.

## Error handling

Provides actionable error messages detailing the failing plugin ID, symbol, or filesystem path.

## Testing

Covered by unit tests across `scanner`, `instance`, and `host`.

## Known limits

- Does not catch low-level POSIX signals (e.g. SIGSEGV) generated inside C/C++ plugins.
