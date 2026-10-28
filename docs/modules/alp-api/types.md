# `alp-api` / `types`

## Purpose

Defines shared data types, permission tiers, summaries, and timestamp formatting utilities for `alp-api`.

## Public API

- `PermissionTier`: `Observe`, `Edit`, `Export`.
- `ApprovalMode`: `Auto`, `ConfirmDestructive`, `Review`.
- `ProjectSummary`: project overview and statistics.
- `TrackSummary`: track state and volume/pan.
- `ClipSummary`: clip boundaries and fades.
- `AnalysisReport`: loudness and quality analysis.
- `format_timestamp(secs) -> String`: converts seconds to `hh:mm:ss.mmm`.

## Data flow

1. Summaries are constructed from internal domain models and serialized into JSON for RPC consumers.
2. Permission tiers are compared using partial ordering.

## Allowed dependencies

- `serde::{Deserialize, Serialize}`

## Threading / real-time notes

Pure data structures and string formatting utilities with zero runtime overhead.

## Error handling

Immutable structs with validated constructors.

## Testing

Covered by unit tests verifying permission tier hierarchies and timestamp conversions.

## Known limits

- None.
