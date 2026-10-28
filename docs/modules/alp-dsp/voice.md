# `alp-dsp` / `voice`

## Purpose

Specialized vocal cleanup processors tailored for spoken word, podcast, and voice-over tracks:
- `DeEsser`: High-frequency sibilance suppression
- `DeHum`: Mains hum attenuation at 50 Hz / 60 Hz and harmonics
- `DeClick`: Transient click and mouth noise removal

## Public API

- `DeEsser`: `new`, `voice_default`, `prepare`, `process`, `reset`, `name`
- `DeHum`: `new`, `voice_default`, `prepare`, `process`, `reset`, `name`
- `DeClick`: `new`, `voice_default`, `prepare`, `process`, `reset`, `name`
- All three implement `crate::Effect`

## Data flow

1. **DeEsser**: Bandpass sidechain filter detects sibilance energy; applies proportional dynamic gain reduction when exceeding threshold.
2. **DeHum**: Series cascade of narrow notch biquad filters tuned to fundamental mains frequency (50/60 Hz) and 2nd, 3rd, and 4th harmonics.
3. **DeClick**: Evaluates sample-to-sample derivative; when steep transients exceed statistical threshold, repairs sample using cubic/linear polynomial interpolation from adjacent clean samples.

## Allowed dependencies

- `alp_core::CoreError`
- `crate::Effect`
- `crate::biquad::{Biquad, FilterType}`
- Standard library math

## Threading / real-time notes

All three processors are zero-allocation and lock-free in `process`, safe for execution inside the real-time audio callback. Filter buffers are allocated once during `prepare`.

## Error handling

Constructors validate numerical boundaries (frequencies, thresholds, ratios) and return `CoreError::Validation` if invalid.

## Testing

Unit tests in `voice.rs` verify:
- Attenuation of sibilance frequencies by DeEsser without affecting mid frequencies.
- Attenuation of 50 Hz / 60 Hz hum components by DeHum.
- Detection and repair of isolated transient spikes by DeClick.

## Known limits

- De-Clicker uses localized 1st/2nd derivative heuristics; complex vinyl crackle bursts may require multi-band AR modeling.
