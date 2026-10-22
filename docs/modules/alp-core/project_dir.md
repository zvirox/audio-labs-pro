# `alp-core::project_dir`

## Purpose

Owns the `*.alp/` directory layout and the load/save cycle. It is the only place
that knows where `project.json`, `media/`, `peaks/`, `renders/` and the
`autosave/` recovery copy live.

## Public API

- `ProjectPaths::new(root)`, `root`, `project_file`, `media_dir`, `peaks_dir`,
  `renders_dir`, `autosave_file`, `has_autosave`
- `resolve_relative(relative)`
- `ensure_layout`, `create(project)`, `open(path)`, `load`, `save(project)`
- `autosave(project)`, `recover_autosave()`, `discard_autosave()`

## Data flow

`open` accepts either a project directory or a `project.json` path, resolves the
root, reads the file and deserialises it. `save` validates, serialises the
versioned JSON envelope, writes atomically and removes any recovery copy.
`autosave` writes the same envelope to `autosave/project.json` without touching
the clean file. `recover_autosave` validates that copy and only then promotes it
to `project.json`. `create` makes the layout and saves. `ensure_layout` is
idempotent.

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
rename and reopen it. A second test rejects parent and absolute paths. Additional
tests simulate an unsaved edit, recover it, confirm the clean save clears the
copy, and check that a corrupt autosave is kept and reported.

## Known limits

- No project lock; two processes can overwrite each other.
- Autosave is explicit in the core API. The desktop session opens a fixed
  `ui-demo` project and debounces autosaves; there is no project picker yet.
- Relative paths are accepted as strings, not canonicalised on save.
