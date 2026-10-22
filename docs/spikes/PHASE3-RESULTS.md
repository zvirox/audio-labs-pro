# Phase 3 results — recording and single-track editing

Status: complete on this machine; headless harness, 30-minute capture soak and forced-kill recovery all pass.

## Environment

- Linux, Rust stable, release `alp-cli`
- No network access used by the harness
- Hardware checks require an input device and are opt-in

## Automated harness

```sh
bash scripts/phase3-demo.sh
```

The default run:

1. creates a project and imports a 48 kHz and a 44.1 kHz source;
2. converts the 44.1 kHz source to the project rate;
3. writes an autosave, applies a ripple delete to autosave only, then recovers
   the autosave and checks that the clean project file was not touched;
4. renders a float32 reference, exports 24-bit WAV and 24-bit FLAC with TPDF
   dither and tags;
5. null-tests both exports against the reference.

Observed on this machine:

| Check | Result |
|---|---|
| import | 2 clips · 5.000 s · both sources 48 kHz |
| autosave recovery | clean file 3 clips · recovered edit 4 clips |
| reference render | 480,000 frames · 10.000 s · peak 0.1793 |
| WAV export | 24-bit · 0.240 s wall time |
| FLAC export | 24-bit · 0.118 s wall time |
| WAV null test | max difference 1.8e-7 |
| FLAC null test | max difference 1.8e-7 |
| null threshold | 0.001 |
| UI smoke | `ALP_AUTOCLOSE_SECS=3 ALP_FORCE_TINT=1 cargo run -p alp-ui` exits cleanly |
| UI tests | background import, punch-in, trim, envelope dip, autosave recovery and undo |
| UI autosave | `ui-demo` creates `project.json` + `media/demo-take.wav`; reopen loads it |

## Hardware checks

```sh
bash scripts/phase3-demo.sh --record
bash scripts/phase3-demo.sh --soak --minutes 30 --record
```

`--record` starts a long capture in the background, kills it with `SIGKILL`
after eight seconds, recovers the spool and checks that at least one second of
audio was finalized. `--soak` records for the requested number of minutes and
fails if fewer than one second is missing or any sample was dropped.

Observed on the system default 48 kHz stereo input:

| Check | Result |
|---|---|
| device | system default input · 48 kHz · 2 ch |
| forced-kill recovery | 379,904 frames recovered after `SIGKILL` · `recovered=true` |
| 30-minute soak | 86,401,536 frames · 1,800.032 s · 168,753 callbacks · 0 dropped samples |

## Known limits found during implementation

- Autosave recovery is implemented in `ProjectPaths`, the CLI and the desktop
  `ui-demo` session. The desktop path is fixed; there is no project picker.
- Capture monitoring is a nearest-frame copy; it is not a filtered resampler.
- Normalization uses the source-level peak rather than a per-clip measured peak.
- Import in the CLI decodes and converts synchronously. The UI decodes dropped
  files on a worker but imports a recording synchronously after the take stops.
- Punch-in trims or pads the take to the selected range; it does not arm automatic
  stop at the selection end.
- A 30-minute stereo 48 kHz f32 capture needs about 1.5 GB free for the spool and
  final WAV. The harness workspace follows `TMPDIR`; point it at a larger disk if
  `/tmp` is small.

## Capture reliability

The first hardware attempts exposed two harness/capture-path issues:

- Finalizing the 30-minute WAV hit `/tmp`'s disk quota. The spool alone held
  exactly 86,400,000 frames, and a recovery on a larger disk finalized and
  decoded it cleanly.
- The 2-second capture ring overflowed while the writer synced chunks, dropping
  380,416 samples over 30 minutes. The ring is now 10 seconds and the writer
  polls at 1 ms; the final soak recorded 86,401,536 frames with zero drops.
