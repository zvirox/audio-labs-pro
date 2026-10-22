# `alp-cli::commands::project`

## Purpose

Implements project creation, import, inspection, offline render and playback.
It is the bridge between the JSON project model and the audio engine.

## Public API

- `create(directory, name, sample_rate)`
- `import(project_path, input, track_name, at_seconds, display_name)`
- `info(project_path, json)`
- `render(project_path, output, bit_depth)`
- `play(project_path, seconds)`
- `default_render_path(paths, name)` (helper)

## Data flow

`create` opens a new `ProjectPaths`, adds a `Track 1` through `AddTrack`, and
saves. `import` probes the file, copies it to `media/<source-id>-<name>`, adds a
source and a clip at the requested timeline position, and saves atomically.
`info` prints metadata. `render`/`play` load every source with `alp-media`,
build `SourceAudio`, convert to a `MixScene` and call the engine. All structural
edits go through `CommandStack`.

## Allowed dependencies

`alp-core`, `alp-media`, `alp-engine`, `anyhow`, `serde_json`, `arc`, `std::path`.

## Threading / real-time notes

Synchronous. Import and render may block and allocate heavily. Playback delegates
to the engine; the CLI sleeps while the callback runs.

## Error handling

Existing `project.json`, missing tracks, empty audio, copy failures, invalid bit
depth and engine errors are all returned with context. A source whose stored
metadata differs from the decoded file only logs a warning and uses the decoded
format. A runtime stream error makes `play` fail after printing the report.

## Testing

The demo script creates a project, imports eight overlapping clips, renders it,
and verifies the output duration. `cargo test --workspace` covers the model and
engine beneath it.

## Known limits

- Import appends one clip and cannot trim or split.
- Only the first track is used unless `--track` names an existing one.
- Render holds the whole mix in memory.
- Playback has no transport UI or seek.
