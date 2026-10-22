# `alp-media` / `dither`

## Purpose

Deterministic integer quantization with optional TPDF dither before writing
16/24-bit WAV or FLAC output.

## Public API

- `DitherMode::{None, Triangular}` with `FromStr` and `label`
- `Quantizer::new(max, dither)` and `quantize(sample)` (crate-internal)

## Data flow

Each export creates one `Quantizer`. It seeds a private xorshift state, adds the
difference of two uniform values at one-LSB peak amplitude, rounds and clamps to
the integer range. Identical samples and settings produce identical bits.

## Allowed dependencies

`alp-media` error types only.

## Threading / real-time notes

One quantizer per export, used on the worker thread. It is never used in the
audio callback.

## Error handling

Unknown dither names are rejected by `DitherMode::from_str`.

## Testing

Tests cover deterministic output, non-zero dither on silence, rounding and
negative full-scale clamping.

## Known limits

- Dither is 1-LSB TPDF only; noise shaping is not implemented.
- The noise state is not user-seeded.
