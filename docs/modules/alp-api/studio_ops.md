# `alp-api` / `studio_ops`

## Purpose

Automation wrappers for studio features that previously existed only in the desktop session layer: EQ response telemetry, ACX QA, native M4B export, and three timeline commands.

## Public API

- `EqBandDto`, `EqResponseDto`, `compute_eq_response(bands, sample_rate, points)`.
- `AcxChapterFileDto`, `ApiSession::acx_qa_analyze(&[AcxChapterFileDto])`.
- `M4bChapterDto`, `M4bMetadataDto`, `M4bExportRequestDto`, `ApiSession::audiobook_export_m4b(&request)`.
- `ApiSession::marker_remove(id)`, `loop_range_set(start_secs, end_secs)`, `clip_trim(clip_id, source_start_frame, source_end_frame, timeline_start_frame)`.

## Data flow

1. Check the required permission tier (Observe for analysis, Edit for timeline edits, Export for M4B).
2. Canonicalise every file path through `WorkspaceSandbox::validate_path`.
3. Delegate to pure existing implementations: `alp_dsp::Coefficients`, `analyze_acx_chapters`, `alp_media::export_m4b`.
4. Apply timeline changes through `ApiSession::execute_command`, so they are audited and undoable.

## Allowed dependencies

`alp-core` (commands, ranges, ids), `alp-dsp` (biquad coefficients), `alp-media` (decode/export), `serde`, and existing `alp-api` helpers. No new crate dependency.

## Threading / real-time notes

All operations are synchronous, CPU or filesystem bound, and run on automation worker threads. None touches the real-time audio callback; EQ response is a pure mathematical function of its arguments.

## Error handling

Permission, sandbox-escape, invalid id, missing entity, decode and export failures return typed `ApiError` variants. Timeline commands inherit `alp_core` validation through `execute_command`.

## Testing

Tests cover unity response for disabled bands, real ACX analysis of a generated WAV, marker removal and loop clearing with undo, and a real chaptered M4B export.

## Known limits

- `clip_trim` accepts frame indices rather than seconds, avoiding ambiguity about source vs project sample rates.
- `acx_qa_analyze` decodes whole files into memory; long chapters are bounded by available RAM.
- M4B export inherits the encoder capabilities compiled into `alp_media`; no cover resizing is performed.
