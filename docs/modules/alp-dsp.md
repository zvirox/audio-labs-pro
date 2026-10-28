# `alp-dsp`

## Purpose

Defines the digital signal processing effect boundaries and real-time audio processors shared by live playback, offline rendering, and the command-line interface. Provides biquad filtering, parametric EQ, dynamics (compressor, noise gate, limiter), specialized vocal cleanup (de-esser, de-hum, de-click), ITU-R BS.1770 LUFS loudness metering, and composable effect chains.

## Public API

- `Effect` trait: `name`, `prepare`, `process`, `reset`
- `Gain`: linear gain stage
- `Biquad`, `FilterType`: RBJ biquad filters (HighPass, LowPass, Peaking, Shelves, Notch, BandPass)
- `ParametricEq`, `EqBand`: multi-band parametric equalizer
- `Compressor`: dynamic range compressor with soft-knee and makeup gain
- `NoiseGate`: downward noise gate with attack/hold/release envelopes
- `Limiter`: peak limiter with fast attack and smooth recovery
- `DeEsser`: sibilance detector and attenuator
- `DeHum`: multi-harmonic mains hum notch filter (50/60 Hz)
- `DeClick`: transient derivative spike detector and polynomial interpolator
- `KWeighting`, `measure_lufs`, `calculate_normalization_gain`: BS.1770 loudness measurement and normalization
- `EffectChain`, `VoiceCleanup`: composite effect execution and one-click vocal cleanup chain

## Data flow

The audio playback engine and offline renderers call `prepare(sample_rate, channels)` before audio streaming, followed by zero-allocation `process(&mut [f32])` passes on interleaved blocks in place. `reset` flushes filter state, envelopes, and delay lines on transport seek.

## Allowed dependencies

- `alp-core` (for `CoreError`)
- Standard library math

## Threading / real-time notes

`Effect::process` runs directly on the audio callback. All implementations are strictly zero-allocation, lock-free, and non-blocking. Filter buffers and internal channel states are pre-allocated during `prepare` or construction.

## Error handling

Constructors validate parameters and return `CoreError::Validation` if invalid or non-finite. Block-level processing is infallible and handles denormals.

## Testing

Unit tests across all submodules verify frequency-domain attenuation, dynamics ballistics, transient repair, LUFS calculation accuracy, and limiter ceiling compliance. An automated null test in `scripts/phase5-demo.sh` verifies processing determinism.

## Known limits

- Mono/stereo interleaved processing (surround formats not yet implemented).
- Latency reporting and lookahead delay compensation will be expanded in future phases.
