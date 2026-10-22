# `alp-core::project_dir`

## Purpose

Owns the `*.alp/` directory layout and the load/save cycle. It is the only place
that knows where `project.json`, `media/`, `peaks/` and `renders/` live.

## Public API

- `ProjectPaths::new(root)`, `root`, `project_file`, `media_dir`, `peaks_dir`,
  `renders_dir`
- `resolve_relative(relative)`
- `ensure_layout`, `create(project)`, `open(path)`, `load`, `save(project)`

## Data flow

`open` accepts either a project directory or a `project.json` path, resolves the
root, reads the file and deserialises it. `save` validates, serialises the
versioned JSON envelope and writes atomically. `create` makes the directory
layout and then saves. `ensure_layout` is idempotent.

## Allowed dependencies

`std::fs`, `std::path`, `schema`, `Project`, `CoreError`. No other ALP crate.

## Threading / real-time notes

All operations are blocking filesystem work. Use a worker thread for load/save
on large projects; never from the audio callback.

## Error handling

Missing directories, unreadable files, malformed JSON, unsupported schema
versions and validation failures all return `CoreError`. `resolve_relative`
rejects absolute paths and paths that escape the project root.

## Testing

Tests create a temp project, verify the media and peak directories, save a
rename and reopen it. A second test rejects parent and absolute paths.

## Known limits

- No project lock; two processes can overwrite each other.
- No autosave or crash recovery yet.
- Relative paths are accepted as strings, not canonicalised on save.
