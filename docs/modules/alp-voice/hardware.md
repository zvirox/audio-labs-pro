# `alp-voice` / `hardware`

## Purpose

Provides host hardware capability probing (RAM, CPU cores, GPU/DRM accelerators) to recommend appropriate model tiers ("Fast" vs "Best") and issue warnings prior to heavy model downloads.

## Public API

- `ModelTier`: `Fast`, `Best`, `as_str`
- `HardwareProfile`: `probe`, `total_ram_gb`, `available_ram_gb`, `evaluate_model_fit`

## Data flow

1. `HardwareProfile::probe` reads memory from `/proc/meminfo`, core counts from `std::thread::available_parallelism()`, and DRM/NVIDIA accelerator presence.
2. `evaluate_model_fit` estimates model working RAM and issues actionable warnings if working sets exceed available memory.

## Allowed dependencies

- `serde`, Standard library (`std::fs`, `std::path::Path`)

## Threading / real-time notes

Probes run once during startup or CLI invocation on non-realtime threads.

## Error handling

Missing or unreadable `/proc/meminfo` paths fall back safely to 8 GB default assumptions.

## Testing

Unit tests in `hardware.rs` verify profile probing and threshold warning evaluation.

## Known limits

- Linux-focused probe (/proc/meminfo and /dev/dri); returns conservative defaults on unsupported platforms.
