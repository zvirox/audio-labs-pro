# `alp-dsp` / `denoise`

## Purpose

Noise reduction processors for speech and dialogue:
- `RnnoiseDenoiser`: Deep neural network suppression (using RNNoise via `nnnoiseless`) specialized for vocal tracks, removing non-stationary environmental noise (keyboard clicks, HVAC rumble, traffic) while preserving speech timbre.
- `SpectralDenoiser`: Classical STFT-based spectral subtraction with captured background noise profile (using `realfft`), with over-subtraction and spectral floor protection against musical noise artifacts.
- `NoiseReduction`: Unified `Effect` trait wrapper providing mode switching between neural RNNoise and classical spectral subtraction.

## Public API

- `DenoiseMode`: Enum (`Rnnoise`, `Spectral`) with `Default` implementation.
- `RnnoiseDenoiser`: `new`, `prepare`, `process`, `reset`, `name`, `set_wet`, `wet`. Implements `crate::Effect`.
- `SpectralDenoiser`: `new`, `prepare`, `process`, `reset`, `name`, `capture_noise_profile`, `has_noise_profile`, `set_over_subtraction`, `set_spectral_floor`, `set_strength`, `strength`. Implements `crate::Effect`.
- `NoiseReduction`: `new`, `rnnoise`, `spectral`, `prepare`, `process`, `reset`, `name`, `mode`, `set_mode`, `rnnoise_processor`, `rnnoise_processor_mut`, `spectral_processor`, `spectral_processor_mut`, `capture_noise_profile`. Implements `crate::Effect`.
- Constants: `RNNOISE_FRAME_SIZE` (480), `SPECTRAL_FFT_SIZE` (1024), `SPECTRAL_HOP_SIZE` (512), `SPECTRAL_BINS` (513).

## Data flow

1. **RnnoiseDenoiser**: Incoming interleaved audio samples are de-interleaved per channel into input FIFO buffers. When 480 samples (10 ms at 48 kHz) accumulate, samples are scaled to 16-bit signed PCM range, processed by `nnnoiseless::DenoiseState::process_frame`, scaled back to normalized floating-point range, and pushed to output FIFO buffers. The caller receives the exact requested sample count with fixed 480-sample latency compensation.
2. **SpectralDenoiser**:
   - *Profile Capture*: Analyzes noise-only audio segments with 1024-point Hann-windowed forward FFTs, accumulating the mean magnitude across all frequency bins.
   - *Spectral Subtraction*: Analyzes streaming audio in 1024-point Hann-windowed frames with 50% overlap. Frequency bins are subtracted against the captured profile using an over-subtraction factor $\alpha$ and clamped to spectral floor $\beta$. Inverse FFT reconstructs audio into an overlap-add accumulator with perfect reconstruction under unity gain.
3. **NoiseReduction**: Routes `prepare`, `process`, and `reset` calls dynamically to the active `DenoiseMode`.

## Allowed dependencies

- `crate::Effect`
- `nnnoiseless` (`DenoiseState`, `FRAME_SIZE`)
- `realfft` (`RealFftPlanner`, `RealToComplex`, `ComplexToReal`, `num_complex::Complex32`)
- Standard library (`std::collections::VecDeque`, `std::sync::Arc`)

## Threading / real-time notes

- `prepare` allocates and primes internal FIFO queues, OLA buffers, and FFT scratch buffers.
- Once prepared, `process` operates with fixed-size ring buffers / `VecDeque` capacities without re-allocation during streaming.
- All three structs implement `Send` and are safe for execution on dedicated audio processing threads.

## Error handling

- Invalid parameter values (negative gain, non-finite values) are clamped to safe valid ranges.
- Short or empty audio slices are handled without panic or allocation.
- In `SpectralDenoiser`, if no noise profile has been captured, the processor acts as a clean pass-through.

## Testing

Unit tests in `denoise.rs` verify:
- Processing arbitrary chunk sizes (64, 128, 256, 480, 512, 1024 samples) through RNNoise without panics or clipping.
- Stereo multi-channel processing in RNNoise.
- Reduction of RMS noise energy by `SpectralDenoiser` following profile capture.
- Passthrough behavior of `SpectralDenoiser` prior to capturing a noise profile.
- Mode switching between RNNoise and Spectral in `NoiseReduction`.

## Known limits

- RNNoise neural weights are trained natively on 48 kHz speech; processing at divergent sample rates operates on equivalent block sizes.
- `SpectralDenoiser` requires a stationary noise segment for profile capture; non-stationary impulses are better addressed by `RnnoiseDenoiser` or `DeClick`.
