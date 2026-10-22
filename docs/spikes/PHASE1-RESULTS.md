# Phase 1 results — foundation acceptance

- Date: 2026-10-22
- Build: release (`cargo build --release -p alp-cli`)
- Machine: Arch Linux x86_64, kernel 7.2.11, PipeWire/ALSA default device
- Toolchain: rustc 1.99.1, edition 2024

## Verdict

Phase 1 meets the build-plan acceptance criteria on this machine. The project
model and undo/redo, media pipeline, real-time engine, CLI, guards and CI are
in place. The 10-minute soak ran eight simultaneous clips with no reported
stream error and a continuous callback stream.

## Acceptance summary

| Criterion | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `cargo test --workspace` | 57 tests pass (20 core unit, 2 core property, 1 dsp, 19 engine, 13 media, 1 ui-kit, 1 doctest) |
| `scripts/check-deps.sh` | pass (9 crates, directions match the architecture) |
| `scripts/check-file-length.sh` | pass (no file over 1,500 lines; `commands.rs` 1,274-line warning) |
| `scripts/check-docs.sh` | pass (42 module documents) |
| 1-hour WAV peak cache | miss 1.42–1.54 s, hit 0.013–0.015 s |
| 8 simultaneous clips, 60 s render | 2,880,000 frames in 1.50 s |
| 10 s device playback | 931 callbacks, 476,672 frames, no stream error |
| 10-minute soak, 8 clips | 56,244 callbacks, 28,796,928 frames, 600.008 s, no stream error |
| Undo/redo property test | apply/undo/redo reproduces the project byte-for-byte |

## Method

The harness is `scripts/phase1-demo.sh`:

```sh
scripts/phase1-demo.sh          # peaks, eight-clip render, inspection
scripts/phase1-demo.sh --play   # plus 10 s playback
scripts/phase1-demo.sh --soak   # plus a 10-minute eight-clip playback
scripts/phase1-demo.sh --record # plus a 5 s capture
```

### Peak cache

1. `alp-cli gen` writes a 3,600 s mono 48 kHz 16-bit WAV (172,800,000 frames,
   330 MiB) by streaming samples to a temp file.
2. The first `alp-cli peaks --json` run decodes the WAV and builds three levels
   (256, 1,024 and 4,096 frames per bucket).
3. The second run reopens the `ALPPEAKS` cache and validates magic, version,
   sample rate, frame count and geometry.

Measurements:

| Item | Value |
|---|---|
| source | 172,800,000 frames, 330 MiB |
| cache build (miss) | 1.490–1.537 s |
| cache reopen (hit) | 0.013–0.015 s |
| cache file | 3.4 MiB, 3 levels |

The hit path is dominated by opening the source for fingerprinting and reading
the cache header; no audio is decoded.

### Eight simultaneous clips

Eight copies of a 60 s, 48 kHz stereo 220 Hz tone (amplitude 0.05) are imported
at timeline frame 0 and rendered to float32 WAV through the same `Mixer` used
for playback:

| Item | Value |
|---|---|
| clips | 8 |
| timeline | 60.000 s |
| rendered frames | 2,880,000 |
| wall time | 1.50 s (about 40× realtime) |
| peak | 0.359 |
| RMS | 0.183 |
| output | float32 WAV, exact frame count |

The amplitude keeps the 8× sum below full scale while still proving that all
eight sources are mixed.

### Playback and soak

`--play` plays the 60 s project for 10 s on the default PipeWire/ALSA output
device. The callback reported 931 invocations, 476,672 frames and
`stream_error=false`.

`--soak` uses a 600 s mono source (amplitude 0.01) imported eight times, then
plays with `--seconds 600`:

| Item | Value |
|---|---|
| callbacks | 56,244 |
| frames | 28,796,928 |
| elapsed | 600.008 s |
| average callback period | 10.67 ms |
| stream error | false |
| finished flag | false (the 600 s wall-clock cap was reached six callbacks before the scene end) |

The last 3,072 frames (six device callbacks of 512 frames) were outside the requested cap. This is a harness boundary,
not a mixer or device failure; a longer `--seconds` would let the scene finish
naturally.

## Limitations and follow-ups

- **Xrun visibility:** cpal exposes stream errors but not an ALSA xrun counter.
  The soak proves a continuous callback stream and no backend error; a future
  debug build should add a callback-deadline/underrun counter and an allocator
  guard. ADR 0011 records the allocator guard as a follow-up.
- **Memory:** sources are decoded fully into RAM and playback uses whole-buffer
  resampling. A long, many-track project will need streaming and a clip
  scheduler.
- **Mixed rates:** the final change scales source ranges, fades and envelopes
  when a source rate differs from the project rate, and
  `Project::duration_frames` scales mixed-rate sources. This path has unit
  coverage (`mixed_rate_sources_scale_to_project_frames`); the soak used
  matching 48 kHz sources.
- **Effects and pan:** `alp-dsp` is a boundary only; pan and effect parameters
  are stored but not rendered in Phase 1.
- **Single machine:** all numbers are from one Arch/PipeWire desktop. CI runs
  the headless checks but has no sound card.

## Artifacts

The harness keeps its temporary directory with `--keep`. A run contains the
generated WAV, peak cache, `.alp` project, media copies and the rendered mix.
CI does not run the hardware or hour-long checks; it runs fmt, clippy, tests,
build and the three guards.
