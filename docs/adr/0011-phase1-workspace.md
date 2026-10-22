# 0011 — Phase 1 workspace, crate graph and real-time seam

- Status: accepted
- Date: 2026-10-22
- Phase: 1 (foundation)

## Context

Phase 0 proved the risky pieces (glass, cpal, peaks, inference) in a throwaway
spike workspace. Phase 1 must turn that evidence into a shipping skeleton with
enforced boundaries before any screen or model is built.

## Decision

- One Cargo workspace, Rust edition 2024, `rust-version = "1.89"`.
- Nine crates: `alp-core`, `alp-media`, `alp-dsp`, `alp-engine`, `alp-voice`,
  `alp-infer`, `alp-ui-kit`, `alp-ui`, `alp-cli`.
- `alp-core` is a leaf. The dependency directions are fixed in
  `docs/architecture/ARCHITECTURE.md` and checked by
  `scripts/check-deps.sh` on every CI run.
- Internal audio is interleaved f32 at the project rate (48 kHz default),
  following ADR 0004. Sources are decoded and resampled before the callback.
  Clip source ranges stay in native source frames; the project and scene scale
  them to project frames so mixed-rate clips keep their real duration.
- The audio callback receives only an immutable `MixScene` snapshot and an
  `rtrb` command queue for control. The Phase 1 playback path constructs the
  mixer before the stream starts.
- `alp-media` owns the `ALPPEAKS` cache (ADR 0009); the UI never parses it.
- Project saves, peak caches, renders and captures use a sibling temp file and
  an atomic rename.
- Every module has a document under `docs/modules/` with the same eight
  sections, checked by `scripts/check-docs.sh`.
- No source file may exceed 1,500 lines; files over 1,000 produce a warning.

## Consequences

- The CLI can exercise the whole stack without a UI, which is why Phase 1 is
  acceptance-tested headlessly.
- `alp-dsp`, `alp-voice`, `alp-infer`, `alp-ui-kit` and `alp-ui` are boundaries
  in Phase 1, not implementations. Their manifests and docs exist so later
  phases cannot quietly invert the graph.
- Playback currently decodes all sources into memory and resamples whole
  buffers. Streaming and a clip scheduler are deferred, but the callback
  contract will not change.
- A debug allocation guard for the callback is still a follow-up; the Phase 1
  mixer is allocation-free by construction and reviewed, but there is no
  automated allocator assertion yet.
