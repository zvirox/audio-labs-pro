# ADR 0027: Media Import and Clip Placement over MCP

- **Status:** Accepted
- **Date:** 2026-10-29
- **Deciders:** Audio Labs Pro Engineering
- **Consulted:** ALP_BUILD_PLAN.md Section 10.3 (Media and Tracks/Clips tool rows), ADR 0026 (voice toolset; its "files land in `media/` but are not registered" follow-up), `alp-ui` session import conventions

## Context and Problem Statement

The voice toolset (ADR 0026) made agents able to synthesize speech into the project `media/` directory, but nothing registers those files as project sources, and no MCP tool can place any audio on the timeline. The build plan's Media row (`media_list`, `media_import`) and the Tracks/Clips row (`clip_add` and friends) are unshipped, so the headless narration workflow dead-ends after generation. At the same time, `alp-core` already ships the exact undoable commands the tools need (`AddSource`, `AddClip`), and the UI's importer already establishes the storage convention.

## Decision Drivers

1. **Undoability:** "All actions remain undoable" (build plan 10.5) - imports and clip placements must go through the command stack (`execute_command`), not direct project mutation. The core commands already exist and validate (duplicate ids, missing source, index bounds), so zero `alp-core` changes are needed.
2. **Overwrite safety:** an import must never be able to damage existing media or code outside the sandbox. Storage names embed a fresh `SourceId`, so a new file lands under a name that cannot pre-exist.
3. **Tier mapping:** `media_list` is Observe; `media_import` and `clip_add` are Edit, matching the plan's tool table.
4. **Reuse of convention:** match the UI importer (`media/{source_id}-{slug}.wav`, Float32 WAV, native sample rate preserved, peak measured at import) so `alp-api` and `alp-ui` produce interchangeable projects.

## Considered Options

- **Option 1: `media_list` + `media_import` + `clip_add` now, via existing commands (Chosen)** - completes the generate-to-timeline chain (tts_generate -> media_import -> clip_add) with three bounded tools and zero core changes; `clip_update`, `clip_remove`, `marker_add`, and batch edits stay queued for later slices.
- **Option 2: Auto-register `tts_generate` output as a project source** - rejected for v1: it couples two tools and surprises state changes into a synthesis call. The explicit import step keeps each tool's effect legible and auditable; docs present the two-step chain as the recommended flow.
- **Option 3: Copy files with `fs::copy`, preserving containers (FLAC stays FLAC)** - rejected for v1: the UI convention decodes and re-encodes to Float32 WAV, and alp-api must stay byte-compatible with what the editor already produces. Re-encoding is lossless from the decoded PCM and normalizes the container.
- **Option 4: Let `media_import` write outside `media/` or accept a destination name** - rejected: never needed by the plan, and a client-chosen filename would reopen the overwrite question for no benefit.

## Decision

1. **`media_list` (Observe, `alp-api::media`):** summarize `project.sources` - id, name, relative path, sample rate, channels, duration (frames over the source's own rate; mixed rates are the engine's concern, per `alp-core`'s duration contract), peak, import time, and whether the file is present on disk. No decoding; cheap and read-only.
2. **`media_import` (Edit):** arguments `{path, name?}`. The path is validated by `WorkspaceSandbox` (it may live anywhere inside the workspace roots, not only the project), decoded to prove it is audio, re-encoded as Float32 WAV to `media/{source_id}-{slug}.wav` (fresh `SourceId`; the slug follows the UI's `safe_file_stem` rules - ASCII, lowercase, trimmed, bounded), peak measured from the decoded buffer, and registered with `AddSource` at the end of the source list. The stored sample rate and channel count are the decoded file's own values; the render path already mixes rates. Result: source id, display name, relative path, absolute path, duration, rate, channels, peak.
3. **`clip_add` (Edit, `ApiSession` next to `split_clip`):** arguments `{track_id, source_id, start_secs, duration_secs?, source_offset_secs?, gain?}`. Defaults: the whole remaining source from its head at unity gain. Source range math uses the source's own sample rate; the timeline position uses the project rate. The range is clamped into `[0, source.frames]` and rejected if empty; unknown track or source ids fail with plain-language `NotFound` before the command runs. The clip is appended at the end of the track's clip list via `AddClip`, inheriting its validation.
4. **Error shape:** user-fixable failures (unknown ids, empty ranges, non-audio files, sandbox escapes) are `isError` tool results with plain-language causes in the established style; protocol errors stay reserved for malformed calls.
5. **No new dependencies; the alp-api -> alp-core/alp-media edges are already legal.**

Deferred explicitly to later slices: `media_import` volume/batch mode, `clip_update`/`clip_remove`, `marker_add`, `edit_batch`, and auto-registration of generated TTS files (Option 2 rationale).

## Consequences

**Positive**

- The plan's reference narration workflow becomes expressible end to end through MCP: synthesize, import, place, render, export.
- Imports and placements are undoable and revision-tracked because they ride the existing command stack; the audit log captures them like every other Edit-tool call.
- `alp-api` and the UI now agree on on-disk media conventions, so projects moved between surfaces stay coherent.

**Negative / follow-ups**

- Re-encoding import normalizes everything to Float32 WAV, so a 30 MB FLAC show file doubles in size on import; a passthrough-copy mode is a cheap later optimization.
- `clip_add` appends only; moving/resizing a clip after placement needs `clip_update`/`MoveClip` exposure (next clip-tool slice).
- Overlapping clips are legal at the core layer (the mix engine sums them); agents get no overlap warning in v1 - the response includes the clip's written range so callers can detect it.

## References

- `crates/alp-api/src/media.rs` - media list/import (this slice)
- `crates/alp-api/src/session.rs` - `clip_add`
- `crates/alp-core/src/commands/source.rs`, `commands/clip.rs` - `AddSource`, `AddClip` (reused unchanged)
- `crates/alp-ui/src/session/mod.rs` - the import conventions mirrored here
- ADR 0026 - the voice toolset whose outputs this slice completes
