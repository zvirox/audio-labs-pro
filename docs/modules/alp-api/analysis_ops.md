# `alp-api` / `analysis_ops`

## Purpose

Measures the project: loudness, silence, signal quality, band spectrum,
waveform and spectrogram art, and render comparison. Every number comes from a
real offline render of the project (or of the files named for comparison), and
the two picture tools return the art they computed rather than a locator.

## Public API

Reports (all `Serialize`/`Deserialize`):

- `QualityReport` — `integrated_lufs`, `peak_dbfs`, `noise_floor_dbfs`,
  `clipping_spikes`, `detected_hum_hz`, `active_ratio`.
- `SilenceReport` — `duration_secs`, `silence_count`, `total_silence_secs`,
  `avg_silence_secs`, `max_silence_secs`, `threshold_dbfs`, `min_silence_secs`.
- `SpectrumReport` — `bins` (band centres in Hz), `magnitudes`,
  `magnitudes_dbfs`, `blocks`, `sample_rate`.
- `WaveformRender` — `ascii`, `width`, `height`, `peak`, `duration_secs`.
- `SpectrogramRender` — `ascii`, `width`, `height`, `band_centers_hz`,
  `range_db`.
- `CompareRendersResult` — `similarity`, `differences`, `recommendation`.

Operations on `ApiSession`:

- `analyze_quality(&self) -> Result<QualityReport, ApiError>`
- `analyze_silence(&self) -> Result<SilenceReport, ApiError>`
- `analyze_spectrum(&self) -> Result<SpectrumReport, ApiError>`
- `render_waveform(&self, width: usize, height: usize) -> Result<WaveformRender, ApiError>`
- `render_spectrogram(&self, width: usize, height: usize) -> Result<SpectrogramRender, ApiError>`
- `compare_renders(&self, path_a: &str, path_b: &str) -> Result<CompareRendersResult, ApiError>`

Crate-internal helpers re-used by `session::analyze_loudness`:

- `level_stats(samples, rate, channels) -> LevelStats` — shared peak/noise
  floor/active ratio/clipping maths.
- `LevelStats` — `peak_dbfs`, `noise_floor_dbfs`, `active_ratio`,
  `clipping_spikes`.

## Data flow

1. Check `PermissionTier::Observe`.
2. Render the project through `render_project_mixdown` (decode sources → build
   `alp_engine::MixScene` → offline render). `compare_renders` instead decodes
   the two named files through the sandbox.
3. Measure:
   - Loudness via `alp_dsp::measure_lufs` (BS.1770 K-weighting).
   - Frame levels by folding the buffer into 20 ms chunks and taking RMS;
     the noise floor is the 10th percentile, `active_ratio` the fraction above
     −45 dBFS, clipping a count of `|sample| >= 1.0`.
   - Silence by walking frames and keeping runs whose peak stays under
     −30 dBFS for at least 100 ms.
   - Spectrum/spectrogram by Goertzel probes at log-spaced centres.
   - Waveform by per-column peak envelopes.
4. Return the typed report.

`render_waveform` maps column peaks to a row threshold grid and prints rows
top-down (`(0..rows).rev()`), so a loud passage fills the lower rows. `compare_renders`
reports the sample rates and frame counts of both files alongside the average
and maximum absolute difference.

## Allowed dependencies

- `serde::{Deserialize, Serialize}`
- `alp_core::frames_to_secs`
- `alp_dsp::measure_lufs`
- `alp_media::{decode, PcmBuffer}`
- `crate::error::ApiError`
- `crate::session::ApiSession`
- `crate::types::PermissionTier`

## Threading / real-time notes

Analysis runs synchronously on the caller's thread over an offline render. It
never touches the real-time audio callback, and nothing here allocates on an
audio thread.

## Error handling

- `ApiError::PermissionDenied` when the session tier is below `Observe`.
- `ApiError::Validation` when the render cannot be built or a comparison path
  fails sandbox validation.
- `ApiError::Audio` from decode failures in `compare_renders`.

## Testing

Unit tests in `analysis_ops.rs` cover the percentile helper, `level_stats`
(peak, noise floor, active ratio, clipping), and Goertzel power. End-to-end
behaviour is covered through MCP: `render_waveform_returns_the_art_it_computed`
asserts a tone appears in the columns where it really is, and
`render_spectrogram_returns_a_real_time_frequency_map` asserts a 220 Hz tone
lands in the band nearest 220 Hz.

## Known limits

- `peak_dbfs` is the **sample peak**; inter-sample true peak (oversampled
  measurement) is not implemented.
- `detected_hum_hz` is always `None`: there is no narrowband mains detector yet,
  and reporting a guess would be fabrication.
- Spectrum and spectrogram use Goertzel probes at 16 (spectrum) or `height`
  (spectrogram) log-spaced centres — band energy, not an FFT bin dump.
- Silence analysis uses a fixed −30 dBFS threshold and a fixed 100 ms minimum
  run length; neither is configurable through this API.
- `compare_renders` compares by sample index without resampling, so files with
  different rates or lengths produce a large reported difference by design.
