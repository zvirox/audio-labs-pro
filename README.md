# Audio Labs Pro

A Linux-only, English-only audio studio written in Rust: multitrack
recording/editing/mixing/export plus local AI text-to-speech, dictation and voice
profiles. Fully offline — the only network traffic is user-initiated model
downloads.

- License: [Apache-2.0](LICENSE)
- Platform: Linux (Wayland and X11)
- UI: iced 0.14 with an in-house component kit, frosted-glass floating panels

## Status

Phases 1–3 are complete on this machine. Phase 3 (recording and single-track
editing) is green: crash-safe capture with recovery, input peak/RMS metering
and monitoring, undoable range edits, import conversion, clipboard, trim,
fades/gain/normalize/reverse/invert, envelope dips, punch-in, background
drag/drop import, autosave recovery, and WAV/FLAC export with dither and tags.
The shell exposes these through the transport, inspector and export panels.

Hardware acceptance also passes: a 30-minute 48 kHz stereo capture recorded
86,401,536 frames with **0 dropped samples**, and a capture killed with
`SIGKILL` recovered 379,904 frames. See
[Phase 3 results](docs/spikes/PHASE3-RESULTS.md).

Real compositor blur is implemented for KWin on X11. Wayland and GNOME keep the
legible tint fallback because the Wayland background-effect bridge requires
`unsafe`; see [ADR 0012](docs/adr/0012-phase2-shell-and-glass.md).

See:

- [Documentation index](docs/README.md)
- [Architecture](docs/architecture/ARCHITECTURE.md)
- [Module documentation](docs/modules/)
- [Phase 0 results](docs/spikes/PHASE0-RESULTS.md)
- [Phase 1 results](docs/spikes/PHASE1-RESULTS.md)
- [Phase 2 results](docs/spikes/PHASE2-RESULTS.md)
- [Phase 3 results](docs/spikes/PHASE3-RESULTS.md)
- [Third-party licenses](THIRD_PARTY.md)

## Build and test

Prerequisites on Ubuntu/Arch: a Rust toolchain (1.89+), `pkg-config`, ALSA
headers, and the usual winit/iced graphics libraries (Wayland, XKB, X11, OpenGL,
fontconfig). On Ubuntu the CI list is the reference:

```sh
sudo apt-get install libasound2-dev pkg-config \
  libxkbcommon-dev libwayland-dev wayland-protocols \
  libx11-dev libxcursor-dev libxrandr-dev libxi-dev \
  libgl1-mesa-dev libegl1-mesa-dev libfontconfig1-dev \
  libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
```

```sh
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
bash scripts/check-file-length.sh
bash scripts/check-deps.sh
bash scripts/check-docs.sh
bash scripts/check-ui.sh
```

## Quick start

Open the desktop shell:

```sh
cargo run --release -p alp-ui
```

The shell starts with a synthetic 10-minute waveform, so it needs no audio
device or model. Useful diagnostics:

```sh
ALP_DEMO_SECONDS=3600 ALP_DEMO_AUTOSCROLL=1 ALP_METRICS=1 \
  ALP_AUTOCLOSE_SECS=10 cargo run --release -p alp-ui
```

`ALP_FORCE_TINT=1` disables the KWin X11 blur request. The CLI still exercises
the whole audio stack:

```sh
cargo run -p alp-cli -- gen demo.wav --seconds 5 --channels 2
cargo run -p alp-cli -- decode demo.wav
cargo run -p alp-cli -- peaks demo.wav
cargo run -p alp-cli -- new demo.alp --name "My Project"
cargo run -p alp-cli -- import demo.alp demo.wav
cargo run -p alp-cli -- render demo.alp mix.flac --format flac --bit-depth int24 \
  --dither triangular --title "My mix"
cargo run -p alp-cli -- play demo.alp
cargo run -p alp-cli -- devices
cargo run -p alp-cli -- record take.wav --seconds 10 --chunk-seconds 2
cargo run -p alp-cli -- recover take.wav
cargo run -p alp-cli -- autosave demo.alp
cargo run -p alp-cli -- recover-project demo.alp
cargo run -p alp-cli -- null-test reference.wav mix.wav
```

`scripts/phase1-demo.sh` is the Phase 1 acceptance harness: a one-hour fixture,
peak-cache miss/hit timing, an eight-clip project and an offline render. Add
`--play`, `--soak` (ten minutes) or `--record` for hardware checks.

`scripts/phase3-demo.sh` is the Phase 3 harness: import conversion, autosave
recovery, WAV/FLAC export and null tests. Add `--record` for forced-kill capture
recovery, or `--soak --minutes 30` for the full dropout gate. A 30-minute stereo
soak needs about 1.5 GB free in `$TMPDIR`; set `TMPDIR` to a larger disk if
needed.

The transport playback controls and most panels are still shell demos. The
inspector edits the fixed `ui-demo` project, recording and imports write real
WAVs into its `media/` directory, autosave protects unsaved edits, and export
renders the edited project. A project picker and save-as remain future work.
