# `alp-core` / `clipboard`

## Purpose

Represents a copied timeline selection independently of clip ids. A fragment
keeps the source range, gain, fades, envelope, reverse and invert flags, so
pasting can allocate fresh clip ids without touching the original.

## Public API

- `ClipFragment` with `to_clip`, `len`, `is_empty`
- `Clipboard` with `from_fragments`, `fragments`, `frames`, `is_empty`, `len`
- `copy_range(project, track_id, range) -> Result<Clipboard, CommandError>`

## Data flow

`copy_range` intersects the selection with every clip on one track, slices each
overlap with the same source-offset mapping used by playback, rebases envelope
points and offsets fragments relative to the selection start. Gaps are retained
only through `Clipboard::frames`, which keeps pasted timing stable.

## Allowed dependencies

No `alp-*` crate dependencies beyond `alp-core` itself. Uses `thiserror` types
from the command layer.

## Threading / real-time notes

Pure data and arithmetic. The clipboard is owned by the UI thread and never
reaches the audio callback.

## Error handling

An empty selection or unknown track is rejected. A clip that disappears while
copying is reported rather than producing a partial clipboard.

## Testing

Unit tests cover selection slicing, source-offset rebasing, fade preservation,
envelope clipping and empty/missing-track rejection. Range command tests paste
the result back into the timeline.

## Known limits

- Clipboard data is in memory only; it is not persisted across app restarts.
- Only one track is copied at a time.
