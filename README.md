# Audio Lab Pro

A Linux-only, English-only audio studio written in Rust: multitrack
recording/editing/mixing/export plus local AI text-to-speech, dictation and voice
profiles. Fully offline — the only network traffic is user-initiated model
downloads.

- License: [Apache-2.0](LICENSE)
- Platform: Linux (Wayland and X11)
- UI: iced 0.14 with an in-house component kit, frosted-glass floating panels

## Status

Phase 1 (foundation) and Phase 2 (UI shell and glass system) are complete. The
workspace builds, the project model and audio engine are tested, and the desktop
binary now opens a real transparent shell with bundled Inter, eight floating
panel stubs, drag/resize/z-order/snap/collapse/dock, a GPU canvas waveform,
command palette, shortcuts, Reduce motion, worker jobs and persisted layout.

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
cargo run -p alp-cli -- render demo.alp mix.wav --bit-depth float32
cargo run -p alp-cli -- play demo.alp
cargo run -p alp-cli -- devices
```

`scripts/phase1-demo.sh` is the Phase 1 acceptance harness: a one-hour fixture,
peak-cache miss/hit timing, an eight-clip project and an offline render. Add
`--play`, `--soak` (ten minutes) or `--record` for hardware checks.

Phase 2 remains a shell demo: the transport and panels are stubs, not a working
editor. Recording and editing arrive in Phase 3.
