# Audio Lab Pro — documentation index

Linux-only, English-only, Rust audio studio with local TTS, dictation and voice
profiles. Licensed Apache-2.0.

## Start here

| Document | Purpose |
|---|---|
| [architecture/ARCHITECTURE.md](architecture/ARCHITECTURE.md) | Crates, processes, threading and enforced dependency directions |
| [architecture/STACK.md](architecture/STACK.md) | Chosen crates, versions and licenses |
| [modules/](modules/) | One page per crate and module, with the same required sections |
| [spikes/PHASE0-RESULTS.md](spikes/PHASE0-RESULTS.md) | Phase 0 measurements, evidence and limitations |
| [spikes/PHASE1-RESULTS.md](spikes/PHASE1-RESULTS.md) | Phase 1 acceptance measurements, soak results and limitations |
| [adr/](adr/) | Architecture Decision Records |
| [../THIRD_PARTY.md](../THIRD_PARTY.md) | Every crate, model and asset with its license |
| [../CHANGELOG.md](../CHANGELOG.md) | Release history |

## Phase status

| Phase | State |
|---|---|
| 0 — research spikes and decisions | complete; evidence in `spikes/` |
| 1 — foundation | complete: workspace, core model, media, engine, CLI, guards, CI |
| 2 — shell and editing | next |

## Phase 1 acceptance

Reproduce with `scripts/phase1-demo.sh` (add `--play`, `--soak` or `--record` for
hardware checks):

| Check | Result |
|---|---|
| `cargo fmt`, `clippy -D warnings`, `cargo test --workspace` | green |
| Dependency, file-length and module-doc guards | green |
| 1-hour WAV peak cache | miss 1.42–1.54 s, hit 0.013–0.015 s, 3 levels |
| Eight simultaneous clips, 60 s render | 2,880,000 frames, peak 0.359, 1.5 s |
| 10 s device playback | 931 callbacks, no stream error |
| 10 min soak | see `docs/spikes/PHASE1-RESULTS.md` |

## Conventions

- Code lives in `crates/`; Phase 0 evidence lives in `spikes/` and is never
  linked into the shipping workspace.
- Every module gets a document under `modules/` in the same commit as the code.
  The required sections are Purpose, Public API, Data flow, Allowed
  dependencies, Threading / real-time notes, Error handling, Testing and Known
  limits.
- Decisions that change the architecture are recorded in `adr/` before the code
  that depends on them.
