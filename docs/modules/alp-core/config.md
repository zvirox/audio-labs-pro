# `alp-core::config`

## Purpose

Resolves the per-user application directories (models, voices, projects,
autosave) using XDG conventions so the app never writes into the source tree.

## Public API

- `AppDirs` with `discover`, `with_roots`, `create`
- Accessors `models_dir`, `voices_dir`, `projects_dir`, `autosave_dir`

## Data flow

`discover` honours explicit `ALP_CONFIG_HOME`, `ALP_DATA_HOME` and
`ALP_CACHE_HOME` overrides, then falls back to the `dirs` crate’s XDG paths
with an `alp` suffix. `create` makes the three roots. `with_roots` injects
roots for tests or portable mode.

## Allowed dependencies

Standard library and `thiserror` via `CoreError`. No other ALP crate.

## Threading / real-time notes

Directory discovery and creation are startup work and may block. Never call
these functions from the audio callback.

## Error handling

Missing `HOME` or an unusable path returns `CoreError::Io` or a validation
error. `create` fails if a directory cannot be created.

## Testing

One test builds `AppDirs::with_roots` and checks the derived model and voice
paths without touching the real home directory.

## Known limits

- Linux/XDG only; Windows and macOS paths are out of scope.
- No per-project override or portable mode yet.
