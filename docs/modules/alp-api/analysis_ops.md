# `alp-api` / `analysis_ops`

## Purpose

Provides audio analysis capabilities (loudness, silence, signal quality, frequency spectrum, waveform rendering, spectrogram generation, and render comparison) for `ApiSession`.

## Public API

- `QualityReport`: integrated LUFS, true peak, noise floor, clipping count, hum detection, speech ratio.
- `SilenceReport`: duration, silence count, total silence, average and maximum silence duration.
- `SpectrumReport`: frequency bins, linear and dBFS magnitudes.
- `WaveformRender`: width, height, and data URI.
- `SpectrogramRender`: width, height, and data URI.
- `CompareRendersResult`: similarity score, differences, recommendation.
- `ApiSession::analyze_quality(&self) -> Result<QualityReport, ApiError>`
- `ApiSession::analyze_silence(&self) -> Result<SilenceReport, ApiError>`
- `ApiSession::analyze_spectrum(&self) -> Result<SpectrumReport, ApiError>`
- `ApiSession::render_waveform(&self, width: usize, height: usize) -> Result<WaveformRender, ApiError>`
- `ApiSession::render_spectrogram(&self, width: usize, height: usize) -> Result<SpectrogramRender, ApiError>`
- `ApiSession::compare_renders(&self, path_a: &str, path_b: &str) -> Result<CompareRendersResult, ApiError>`

## Data flow

1. Verifies caller permissions under `PermissionTier::Observe`.
2. Mixes down project or decodes target audio into a PCM buffer via `render_project_mixdown` or `alp_media::decode`.
3. Performs analysis math:
   - BS.1770 K-weighted filtering and gating for LUFS.
   - Threshold scans for silence intervals and clipping spikes.
   - FFT/magnitude bins for spectrum and spectrogram visualizations.
   - Sample-difference comparisons for render verification.
4. Returns a typed report structure.

## Allowed dependencies

- `serde::{Deserialize, Serialize}`
- `alp_core::frames_to_secs`
- `alp_dsp::measure_lufs`
- `crate::error::ApiError`
- `crate::session::ApiSession`
- `crate::types::PermissionTier`

## Threading / real-time notes

All analysis executes on caller/worker threads using offline renders; never on real-time audio callback threads.

## Error handling

Returns `ApiError::PermissionDenied` if the session tier is insufficient, or `ApiError::Audio` on decode/render failures.

## Testing

Covered by unit tests in `alp-api` and MCP server tool tests in `alp-mcp`.

## Known limits

- Silence detection threshold is currently fixed at -30 dBFS.
