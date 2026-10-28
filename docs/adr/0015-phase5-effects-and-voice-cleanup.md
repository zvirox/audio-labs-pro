# 0015 — Effects, voice cleanup pipeline, and loudness metering

- Status: accepted
- Date: 2026-10-27
- Phase: 5 (Effects and voice cleanup / Release v0.1 "Editor")

## Context

Phase 5 delivers the essential vocal cleanup, dynamic control, and loudness normalization features required for professional voice-over and podcast production. Spoken-word editing requires pristine noise reduction (hum, sibilance, clicks), consistent dynamic levelling without pumping, peak limiting to prevent broadcast inter-sample clipping, and standardized ITU-R BS.1770 / EBU R128 integrated LUFS metering.

Processing must operate infallibly in the real-time audio thread (zero-allocation, lock-free) as well as headlessly in CLI batch scripts and export render paths.

## Decision

### DSP Core and Biquad Architecture

1. Implemented Direct Form I/II biquad digital filters based on Robert Bristow-Johnson's Audio EQ Cookbook formulas in `alp_dsp::biquad`.
2. Created `ParametricEq` featuring a 5-band voice-optimized default topology:
   - Band 1: High-Pass Filter (80 Hz, Q 0.707) for rumble removal
   - Band 2: Low-Shelf (200 Hz, Q 0.707, -1.5 dB) for chestiness control
   - Band 3: Peaking Bell (800 Hz, Q 1.4, -1.0 dB) for boxiness reduction
   - Band 4: Peaking Bell (3.5 kHz, Q 1.5, +1.5 dB) for vocal presence and clarity
   - Band 5: High-Shelf (10.0 kHz, Q 0.707, +1.0 dB) for air

### Dynamics and Limiting

1. Implemented soft-knee `Compressor` with configurable threshold, ratio, attack, release, and makeup gain with decoupled envelope smoothing.
2. Implemented `NoiseGate` with threshold, attack, hold, and release ballistics to silence room tone and breath gaps naturally without chatter.
3. Implemented `Limiter` peak stage ensuring signals strictly respect user ceilings (e.g. -1.0 dB FS for podcast/streaming delivery).

### Voice Cleanup Processors

1. **De-Esser (`DeEsser`)**: Employs a sidechain bandpass detector (default 6.0 kHz) to monitor sibilance energy and dynamically attenuate harsh 's', 'sh', and 't' bursts without darkening stationary vocal tone.
2. **De-Hum (`DeHum`)**: Implements an 8-stage cascaded notch filter tackling fundamental mains hum (50 Hz or 60 Hz) along with its 2nd, 3rd, and 4th harmonics.
3. **De-Click (`DeClick`)**: Transient derivative detector identifying single-sample or few-sample mouth clicks and pops, repairing damaged samples via local polynomial interpolation.
4. **Voice Cleanup Composite (`VoiceCleanup::build`)**: Chained one-click preset running: De-Hum -> De-Click -> High-Pass/Voice EQ -> De-Esser -> Compressor -> Peak Limiter.

### Loudness Metering and Normalization

1. Implemented ITU-R BS.1770-4 K-weighting pre-filter (stage 1 high-shelf acoustic head model + stage 2 high-pass RLB filter).
2. Integrated `measure_lufs` and `calculate_normalization_gain` computing target loudness (e.g. -16 LUFS for podcasts, -14 LUFS for streaming) clamped against maximum true-peak ceilings.
3. CLI exposure via `alp-cli lufs <file>` and `alp-cli cleanup <input> <output>`.

## Consequences

- Real-time audio safety is preserved: all buffer allocations and filter topologies are prepared upfront; block processing is strictly zero-allocation and lock-free.
- Spoken audio files pass automated acceptance tests (`phase5-demo.sh`), reducing hum by >10 dB, attenuating sibilance, removing transient spikes, and verifying limiter ceiling compliance.
- DSP determinism verified with a perfect 0.00000000 null-test between successive runs.
