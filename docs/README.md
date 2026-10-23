# Audio Labs Pro — documentation index

Linux-only, English-only, Rust audio studio with local TTS, dictation and voice
profiles. Licensed Apache-2.0.

## Start here

| Document | Purpose |
|---|---|
| [architecture/ARCHITECTURE.md](architecture/ARCHITECTURE.md) | Crates, processes, threading and enforced dependency directions |
| [architecture/STACK.md](architecture/STACK.md) | Chosen crates, versions and licenses |
| [modules/](modules/) | One page per crate and module, with the same required sections |
| [spikes/PHASE0-RESULTS.md](spikes/PHASE0-RESULTS.md) | Phase 0 measurements, evidence and limitations |
| [spikes/PHASE1-RESULTS.md](spikes/PHASE1-RESULTS.md) | Phase 1 acceptance measurements and soak results |
| [spikes/PHASE2-RESULTS.md](spikes/PHASE2-RESULTS.md) | Phase 2 shell, waveform, persistence and glass results |
| [spikes/PHASE3-RESULTS.md](spikes/PHASE3-RESULTS.md) | Phase 3 edit, export and autosave results; hardware gate status |
| [spikes/PHASE4-RESULTS.md](spikes/PHASE4-RESULTS.md) | Phase 4 multitrack results and known limits |
| [adr/](adr/) | Architecture Decision Records |
| [../THIRD_PARTY.md](../THIRD_PARTY.md) | Every crate, model and asset with its license |
| [../CHANGELOG.md](../CHANGELOG.md) | Release history |

## Phase status

| Phase | State |
|---|---|
| 0 — research spikes and decisions | complete; evidence in `spikes/` |
| 1 — foundation | complete: workspace, core model, media, engine, CLI, guards, CI |
| 2 — UI shell and glass system | complete on this machine; KDE/GNOME matrix limitations documented |
| 3 — recording and single-track editing | complete on this machine: edits, capture, export and shell controls; hardware soak and forced-kill recovery pass |
| 4 — multitrack timeline and mixing | in progress: engine pan, master bus, mixer UI, armed imports, crossfade and stems working; capture live and crossfade UI polish pending |

## Phase 3 acceptance

Reproduce the headless part with `bash scripts/phase3-demo.sh`:

| Check | Result |
|---|---|
| `cargo fmt`, `clippy -D warnings`, 144 tests | green |
| Dependency, file-length, module-doc and UI-widget guards | green |
| Import conversion | 44.1 kHz source converted to 48 kHz |
| Autosave recovery | clean project 3 clips; recovered edited project 4 clips |
| WAV/FLAC null test | max difference 1.8e-7 at 24-bit; threshold 0.001 |
| 30-minute dropout soak | 86,401,536 frames · 0 dropped samples |
| Forced-kill capture recovery | 379,904 frames · `recovered=true` |

## Phase 2 acceptance

Reproduce with `cargo test --workspace` and the diagnostic command in
`docs/spikes/PHASE2-RESULTS.md`:

| Check | Result |
|---|---|
| `cargo fmt`, `clippy -D warnings`, 104 tests | green |
| Dependency, file-length, module-doc and UI-widget guards | green |
| 1-hour waveform autoscroll | 60–61 frames/s, 16.6 ms average frame time |
| Idle CPU with nothing animating | 0.00% over a 7 s sample (0 scheduler ticks) |
| Idle RSS | about 86 MB |
| Inter | bundled variable font, family `Inter Variable`, OFL text shipped |
| Panels persist | versioned `workspace.json` + atomic write, unit-tested round-trip |
| Workers do not block UI | cancellable std-thread jobs + non-blocking channel, tested |
| Raw iced buttons in `alp-ui` | none; `scripts/check-ui.sh` enforces |
| Glass | KWin/X11 property path; tint fallback on Wayland/GNOME |

## Conventions

- Code lives in `crates/`; Phase 0 evidence lives in `spikes/` and is never
  linked into the shipping workspace.
- Every module gets a document under `modules/` in the same commit as the code.
  The required sections are Purpose, Public API, Data flow, Allowed
  dependencies, Threading / real-time notes, Error handling, Testing and Known
  limits.
- Decisions that change the architecture are recorded in `adr/` before the code
  that depends on them.
