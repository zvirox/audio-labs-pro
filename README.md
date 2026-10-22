# Audio Lab Pro

A Linux-only, English-only audio studio written in Rust: multitrack
recording/editing/mixing/export plus local AI text-to-speech, dictation and voice
profiles. Fully offline — the only network traffic is user-initiated model
downloads.

- License: [Apache-2.0](LICENSE)
- Platform: Linux (Wayland and X11)
- UI: iced 0.14 with an in-house component kit, frosted-glass floating panels

## Status

Phase 1 (foundation) is complete: the workspace builds, the project model and
undo/redo are tested, decode/resample/peaks/WAV work, the real-time engine plays
and records, and `alp-cli` exercises the whole stack. Phase 2 starts the glass
shell and editing UI.

See:

- [Documentation index](docs/README.md)
- [Architecture](docs/architecture/ARCHITECTURE.md)
- [Module documentation](docs/modules/)
- [Phase 0 results](docs/spikes/PHASE0-RESULTS.md)
- [Phase 1 results](docs/spikes/PHASE1-RESULTS.md)
- [Third-party licenses](THIRD_PARTY.md)

## Build and test

Prerequisites on Ubuntu/Arch: a Rust toolchain (1.89+), `pkg-config` and ALSA
headers (`libasound2-dev` on Ubuntu, `alsa-lib` on Arch).

```sh
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
bash scripts/check-file-length.sh
bash scripts/check-deps.sh
bash scripts/check-docs.sh
```

## Quick start

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

`scripts/phase1-demo.sh` is the full acceptance harness: a one-hour fixture,
peak-cache miss/hit timing, an eight-clip project and an offline render. Add
`--play`, `--soak` (ten minutes) or `--record` for hardware checks.

The UI and inference binaries are placeholders in Phase 1; their crates exist so
the process and dependency boundaries are enforced from day one.
