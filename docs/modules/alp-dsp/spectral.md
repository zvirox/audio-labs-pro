# `alp-dsp` / `spectral`

## Purpose

High-resolution Short-Time Fourier Transform (STFT) and spectrogram computation engine for Audio Labs Pro:
- `SpectrogramEngine`: Computes time-frequency intensity matrices from PCM audio buffers.
- Configurable FFT analysis window sizes (512, 1024, 2048, 4096, 8192) and hop sizes (typically $N/4$ for 75% overlap or $N/2$ for 50%).
- Classic window functions: Hanning (Hann), Hamming, Blackman-Harris (4-term with -92 dB sidelobe rejection), and Rectangular (boxcar).
- Power spectral density normalization in dBFS, mapped into Linear, Logarithmic (human hearing perception from 20 Hz to 20 kHz), or Mel frequency bin spacing.

## Public API

- `WindowType`: Enum (`Hanning`, `Hamming`, `BlackmanHarris`, `Rectangular`) with `generate_window(size: usize) -> Vec<f32>` and `Default`.
- `FrequencyScale`: Enum (`Linear`, `Logarithmic`, `Mel`) with `Default`.
- `SpectrogramConfig`: Struct with parameters `fft_size`, `hop_size`, `window_type`, `freq_scale`, `min_freq_hz`, `max_freq_hz`, `num_freq_bins`, `min_db`, `max_db`, and validation via `validate()`.
- `SpectrogramData`: Output data matrix holding `sample_rate`, `duration_sec`, `num_frames`, `num_bins`, `freq_bins_hz`, `time_stamps_sec`, `matrix`, `min_db`, `max_db`, with helper functions `get_intensity(frame_idx, bin_idx)` and `intensity_to_db(intensity)`.
- `SpectrogramEngine`: Constructor `new(config: SpectrogramConfig) -> Result<Self, CoreError>`, and transform method `compute(&self, samples: &[f32], sample_rate: u32) -> Result<SpectrogramData, CoreError>`.

## Data flow

1. **Window Generation**: On engine construction, the chosen analysis window of length $N$ (`fft_size`) is generated and normalized by its coherent power sum $\sum w_i^2$.
2. **Frequency Bin Grid**: Target frequency center frequencies are generated according to the requested `FrequencyScale` (linear binning, logarithmic geometric scaling, or Mel band intervals).
3. **Sliding Window FFT**: Input mono PCM samples are partitioned into overlapping frames spaced by `hop_size`. Each frame is windowed in-place and transformed using `realfft::RealToComplex` into $N/2 + 1$ unique complex Fourier bins.
4. **Power Spectral Density & Decibel Scaling**: Magnitude squared $|X[k]|^2$ is converted to dBFS: $10 \log_{10}(P + 10^{-12})$.
5. **Aggregation & Normalization**: The computed power is interpolated/aggregated into the target frequency bins and normalized into floating point range $[0.0, 1.0]$ based on `min_db` and `max_db`.

## Allowed dependencies

- `alp-core::CoreError`
- `realfft` (`RealFftPlanner`, `RealToComplex`, `num_complex::Complex32`)
- `serde` (`Serialize`, `Deserialize`)
- Standard library (`std::f32::consts::PI`, `std::sync::Arc`)

## Threading / real-time notes

- `SpectrogramEngine` plans its FFT forward transform during initialization via `RealFftPlanner::<f32>::plan_fft_forward(fft_size)`.
- The engine implements `Send` and `Sync`, allowing immutable concurrent reuse across background worker threads or worker pools (`alp-session::JobQueue`).
- STFT calculation for long audio buffers is performed asynchronously off the real-time audio thread to preserve zero glitching on the live playback stream.

## Error handling

- Invalid FFT sizes (non-power-of-two or less than 64) and invalid hop sizes return `CoreError::Validation`.
- Empty or zero-sample buffers return an empty `SpectrogramData` payload gracefully without panicking.
- Non-finite or out-of-range decibel levels are clamped to the configured `min_db` and `max_db` boundaries.

## Testing

Unit tests in `spectral.rs` verify:
- Window symmetry and boundary values for Hanning, Hamming, Blackman-Harris, and Rectangular windows.
- Frequency bin monotonicity and scale boundaries across Linear, Logarithmic, and Mel configurations.
- Pure-tone peak detection: feeding synthetic 440 Hz and 1000 Hz sine tones correctly identifies the maximum spectral energy at the corresponding frequency bins.
- Silence handling: digital silence produces flat minimum intensity floor across all bins.
- Configuration validation rejecting non-power-of-two FFT sizes.

## Known limits

- Input audio must be downmixed to single-channel mono prior to analysis (handled automatically by `alp-session` and `alp-api`).
- Target frequency resolution is bounded by the window size $N$ (e.g., 2048 samples at 48 kHz yields $\approx 23.4$ Hz bin resolution).
