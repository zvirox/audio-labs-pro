# Audio Lab Pro — documentation index

Linux-only, English-only, Rust audio studio with local TTS, dictation and voice
profiles. Licensed Apache-2.0.

## Start here

| Document | Purpose |
|---|---|
| [architecture/ARCHITECTURE.md](architecture/ARCHITECTURE.md) | Crates, processes, threading and enforced dependency directions |
| [spikes/PHASE0-RESULTS.md](spikes/PHASE0-RESULTS.md) | Phase 0 measurements, evidence and limitations |
| [adr/](adr/) | Architecture Decision Records (D1–D8 and stack choices) |
| [../THIRD_PARTY.md](../THIRD_PARTY.md) | Every crate, model and asset with its license |
| [../CHANGELOG.md](../CHANGELOG.md) | Release history |

## Phase status

| Phase | State |
|---|---|
| 0 — research spikes and decisions | in progress (spikes run on one Arch/niri machine) |
| 1 — foundation | not started |

## Conventions

- Code lives in `crates/` from Phase 1. Phase 0 evidence lives in `spikes/` and is
  never linked into the shipping workspace.
- Every module gets a document under `modules/` in the same commit as the code.
- Decisions that change the architecture are recorded in `adr/` before the code
  that depends on them.
