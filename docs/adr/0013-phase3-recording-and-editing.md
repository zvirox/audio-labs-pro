# 0013 — Crash-safe capture, range edits and studio export

- Status: accepted
- Date: 2026-10-22
- Phase: 3 (recording and single-track editing)

## Context

Phase 3 adds the first workflows that can lose user work: recording, destructive
timeline edits and file export. The project model already has an undoable
`Command` protocol and an immutable `MixScene`, but it has no recovery format for
a recorder killed mid-take, no range-edit commands and no metadata-aware
WAV/FLAC writer. The recording acceptance gate is a 30-minute take with zero
dropouts plus recovery after a forced kill; editing must stay undoable, and an
export must null-test against the in-app render.

## Decision

### Capture

1. Record raw interleaved f32 frames to `<output>.alp-capture` with a small
   `Recording` handle. A writer thread owns the file; the audio callback only
   pushes into an `rtrb` ring and increments atomics.
2. Keep a JSON manifest at `<output>.alp-capture.json`. It stores the format,
   committed frame count and a finalization flag. The writer syncs the spool and
   updates the manifest at chunk boundaries (`chunk_frames`, default 5 s in the
   UI, configurable in the CLI).
3. `recover_capture` validates the manifest, truncates the spool to the committed
   frame count and writes the WAV atomically. A missing or unrelated manifest is
   rejected; recovery never guesses at raw bytes.
4. Monitoring is optional and separate: input frames are copied into a second
   ring for the output callback. It is a nearest-frame copy, not a resampler, so
   it adds no allocation or lock to either callback.

### Editing

1. Range operations are first-class undoable commands:
   `DeleteRange { ripple }`, `SilenceRange`, `PasteClips`, `DuplicateClip` and
   `PunchIn`. Cut is copy plus ripple delete; the caller supplies the clipboard.
2. Each range command computes the complete replacement clip list once and
   caches it. Redo restores that exact list, so split-generated clip ids stay
   stable across undo/redo.
3. `SilenceRange` keeps later clips in place; `DeleteRange` can ripple or leave a
   gap. A copy is a list of `ClipFragment`s in memory with source ranges, fades,
   envelope points and reverse/invert flags rebased to the selection start.
4. Reverse and phase invert are non-destructive `Clip` booleans. The mixer and
   offline renderer implement both, so preview and export share one path.
   Envelope points are stored relative to the clip and edited through
   `SetClipEnvelope`; the inspector exposes a selection dip and a flat reset.
   Trim maps a timeline selection back through reverse playback before issuing
   `TrimClip`.
5. Normalization uses `Source.peak`, measured once during import. A source
   without a measured peak is rejected rather than normalized from a guess.
   `Clip.reversed`, `Clip.inverted` and `Source.peak` are serde-defaulted so
   schema-1 projects still load.
6. Project recovery is separate from capture recovery: `ProjectPaths` writes
   `autosave/project.json` atomically after edits, and `recover_autosave`
   validates it before replacing `project.json`. A clean save removes the
   recovery copy.

### Export

1. `alp-media::export` owns WAV and FLAC output. It resamples to the requested
   rate, then selects bit depth and deterministic TPDF dither.
2. WAV writes RIFF `INFO` tags; FLAC writes a Vorbis comment block. Both write a
   sibling temporary file and rename atomically.
3. FLAC uses `flacenc` 0.5.1. Its STREAMINFO minimum and maximum block sizes are
   set to the encoder block size because Symphonia 0.6.1 rejects streams whose
   values differ on the short final block. `flac` and `metaflac` accept the same
   file.
4. The UI queues the mix export on the existing worker queue after cloning a
   `MixScene` snapshot. The current project can keep changing while the export
   renders.

## Consequences

- A forced kill loses at most the last capture chunk and at most the last
  autosave interval, never a partially written project or WAV.
- Range edits are deterministic across redo and cannot collide on split ids.
- Preview and export share the mixer, clip flags and effect path. The Phase 3
  harness compares a float reference with 24-bit WAV and FLAC exports; both
  currently pass at a maximum sample difference of 1.8e-7.
- The FLAC workaround is a compatibility choice, not a change to the audio
  samples. It is documented in `alp-media`'s module notes and covered by a
  decode round-trip test.
- Editing is still limited to one track and one selection. Multi-track
  drag/drop, crossfades and armed multitrack recording remain Phase 4.
- Hardware acceptance (30-minute dropout soak and forced-kill capture) is not
  yet run on this machine. `scripts/phase3-demo.sh --soak --record` automates the
  run when an input device is available.
- Project autosave is proven through the CLI and the desktop session. The shell
  opens a fixed `$XDG_DATA_HOME/alp/projects/ui-demo` project, writes media into
  its `media/` directory and flushes an autosave on edits and close. There is no
  project picker or recent-project list yet.
