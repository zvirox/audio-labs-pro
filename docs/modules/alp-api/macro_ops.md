# `alp-api` / `macro_ops`

## Purpose

Automation wrapper that runs an offline DSP macro batch as a tracked API job, with sandbox-checked input/output paths and progress reporting.

## Public API

- `MacroJobReport { job_id, report }`.
- `ApiSession::dsp_macro_execute(&MacroRequest) -> Result<MacroJobReport, ApiError>`.

## Data flow

1. Require the Export tier and reject empty step lists.
2. Canonicalise `input_dir` and `output_dir` through `WorkspaceSandbox::validate_path`.
3. Submit a `JobManager` job and call `alp_engine::process_directory` with a progress callback.
4. Report completion or failure through `job_status`; audit successful calls.

## Allowed dependencies

`alp-engine` (`process_directory`, DTOs), `crate::sandbox`, `crate::jobs`, `serde`. All are existing `alp-api` dependencies.

## Threading / real-time notes

The batch runs synchronously inside the API call but is exposed as a job so MCP clients can poll and cancel it. Cooperative cancellation is checked from the engine's progress callback. Never used on the audio thread.

## Error handling

Empty steps, sandbox escapes, and engine failures return `ApiError::Validation` (or the sandbox error) and mark the job failed. Per-file failures remain in `MacroReport.failed`.

## Testing

Tests verify a one-file gain batch produces an output and a completed job, and that an empty step list is rejected.

## Known limits

- `job_cancel` stops only between files.
- Directories must be inside the workspace sandbox; absolute outside paths are rejected.
- Output is always WAV, inherited from `alp_engine::macro_chain`.
