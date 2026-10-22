# `alp-core::files`

## Purpose

Provides crash-safe writes. No caller writes `project.json` directly; every
save goes through a sibling temp file and an atomic rename.

## Public API

- `create_temp_sibling(target) -> io::Result<(PathBuf, File)>`
- `commit_temp(temp, target) -> io::Result<()>`
- `write_atomic(path, bytes) -> Result<(), CoreError>`

## Data flow

`create_temp_sibling` creates `<name>.<random>.tmp` in the target directory.
The caller writes and flushes. `commit_temp` renames it over the target.
`write_atomic` composes both steps and calls `sync_all` before the rename.

## Allowed dependencies

`std::fs`, `std::io`, `CoreError`. The temp-file naming uses a random suffix
from an internal counter/UUID; no external crate beyond `alp-core`.

## Threading / real-time notes

Blocking filesystem work. Never call from the audio callback. Concurrent writers
to the same path each use a distinct temp file and the last rename wins.

## Error handling

Creation, write, fsync and rename errors are returned as `CoreError::Io`. A
failed write leaves the original file untouched; the temp file is removed by the
caller when appropriate.

## Testing

The atomic-write test writes twice, asserts the target has the new contents and
checks that no `.tmp` sibling remains.

## Known limits

- No directory fsync; a power loss immediately after rename may still lose the
  directory entry on some filesystems.
- Temp files left by a crash are not garbage-collected on startup yet.
