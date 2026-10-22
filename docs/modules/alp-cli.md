# `alp-cli`

## Purpose

A headless harness for Phase 1 and a scripting surface for the project format.
It generates deterministic test audio, decodes and inspects files, builds and
reopens peak caches, imports media into projects, renders projects to WAV, and
lists/plays/records audio devices.

## Public API

Binary `alp-cli` with subcommands:

| Command | Action |
|---|---|
| `gen` | Generate a deterministic tone/noise WAV fixture |
| `decode` | Decode a file and print format, peak and RMS |
| `peaks` | Build or load a peak cache and report hit/miss and timing |
| `new` | Create a project directory with one track |
| `import` | Copy media into a project and add a clip |
| `info` | Print project metadata |
| `render` | Offline-mix a project to WAV |
| `play` | Play a project on the default output device |
| `devices` | List output and input devices |
| `record` | Record the default input device to WAV |

`--json` is available on `decode`, `peaks`, `info` and `devices`. `--log` sets
the tracing filter.

## Data flow

`cli.rs` parses arguments with clap. `commands::dispatch` routes to
`commands::media`, `commands::project` or `commands::device`. Project commands
load through `ProjectPaths`, decode sources through `alp-media`, build a
`MixScene` and call `alp-engine`. Import writes the media copy first, then
applies `AddSource` and `AddClip` through a `CommandStack` and saves atomically.

## Allowed dependencies

`alp-core`, `alp-media`, `alp-engine`, `alp-dsp` and `alp-voice`. External
crates are `clap`, `anyhow`, `serde_json` and `tracing`.

## Threading / real-time notes

The CLI runs synchronously on the main thread. Playback and recording delegate
to the engine, whose callback remains real-time safe. Imports and renders may
block for seconds and are not used from the audio thread.

## Error handling

`anyhow` carries a user-facing context chain; `main` prints `error: ...` with
the full chain and returns exit code 1. Missing devices, missing files, invalid
projects and backend stream errors all fail cleanly. `--json` output is only
printed after the operation succeeds.

## Testing

CI runs `cargo test --workspace` and the guard scripts. `scripts/phase1-demo.sh`
exercises generation, decode, peaks, project creation, import, info and render
end to end, including a one-hour fixture peak-cache miss/hit and an eight-clip
render. `--play` and `--record` add the hardware paths; `devices` is listed by
the CLI but not part of the demo.

## Known limits

- `play` and `record` use only the default device.
- `record` writes float32 WAV and does not resample.
- `render` holds the whole mix in memory.
- Import copies media and appends one clip; trimming and track creation beyond
  the first track are GUI/Phase 2 work.
