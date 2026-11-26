# `alp-session` / `macro_bridge`

## Purpose

Thin desktop bridge that lets the Tauri shell run an offline DSP macro batch through the automation API without adding a direct `alp-api` dependency to `alp-desktop`.

## Public API

- `dsp_macro_execute(&MacroRequest) -> Result<MacroJobReport, String>`.
- Re-exports `MacroRequest`, `MacroStep`, `MacroJobReport`.

## Data flow

1. Resolve the current working directory as the workspace root.
2. Create a sandbox rooted there, an audit log, and a temporary project.
3. Build an Export-tier `alp_api::ApiSession` and call `dsp_macro_execute`.
4. Return the job report or a descriptive string for Tauri.

## Allowed dependencies

`alp-api` (automation session and DTOs) and `alp-core` (project paths). `alp-session` already depends on both.

## Threading / real-time notes

Synchronous and filesystem-bound; call from a Tauri command worker. The API session exists only for the duration of one batch.

## Error handling

Working-directory, sandbox, audit, and API errors are converted to strings. Empty step lists are rejected by the API before any output is written.

## Testing

A test asserts that an empty-step request returns an error.

## Known limits

- The workspace root is the process working directory; multi-root projects are not supported.
- Each invocation creates a fresh API session, so job handles cannot be polled through this bridge; use the MCP tool for job-based workflows.
