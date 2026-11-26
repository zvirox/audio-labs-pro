# `alp-api` / `acx_qa`

## Purpose

Batch ACX audiobook compliance analysis. Validates each chapter's RMS (-23 to -18 dBFS), estimated true peak (<= -3 dBFS), and noise floor (<= -60 dBFS), returning JSON-serializable results for the UI and MCP.

## Public API

- Targets: `ACX_RMS_MIN_DBFS`, `ACX_RMS_MAX_DBFS`, `ACX_TRUE_PEAK_MAX_DBFS`, `ACX_NOISE_FLOOR_MAX_DBFS`.
- `AcxChapterInput<'a>`: `title`, `samples`, `sample_rate`, `channels`.
- `AcxChapterQa`: measured levels, `compliant`, and human-readable `failures`.
- `AcxQaReport`: `chapters`, `compliant_count`, `total_count`, `all_compliant`.
- `analyze_acx_chapters(&[AcxChapterInput]) -> AcxQaReport`.

## Data flow

1. Decode each chapter to interleaved f32 samples (done by the caller).
2. Compute overall RMS, `alp_dsp::measure_true_peak_dbfs`, and the 10th-percentile frame noise floor via `analysis_ops::level_stats`.
3. Compare against ACX thresholds and collect failure reasons.
4. Aggregate counts; an empty batch is not reported as compliant.

## Allowed dependencies

`alp-dsp` for true-peak estimation, `serde` for DTOs, and `crate::analysis_ops` for frame-level noise statistics. No file I/O or UI dependencies.

## Threading / real-time notes

Pure synchronous analysis. Safe to call from an API worker or MCP dispatch; it is not part of the audio callback.

## Error handling

Empty buffers yield `-120 dBFS` levels. Out-of-range or non-finite inputs are clamped by the underlying DSP helpers. No errors are returned; every chapter receives a report so a batch never silently drops a file.

## Testing

Unit tests cover a compliant low-duty-cycle chapter, a full-scale tone that fails all three targets, and an empty batch.

## Known limits

- RMS is measured in dBFS, not gated LUFS; ACX also specifies loudness targets that require integrated measurement.
- Noise floor uses the 10th-percentile 20 ms frame level, which assumes chapters contain genuine quiet passages.
- True peak is the Catmull-Rom quarter-sample approximation from `alp-dsp`.
