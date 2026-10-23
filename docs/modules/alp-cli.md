# `alp-cli`

## Purpose

A headless harness for Phases 1–3 and a scripting surface for the project format.
It generates deterministic test audio, decodes and inspects files, builds and
reopens peak caches, imports and converts media, applies a range edit to
autosave, recovers autosaves, renders WAV/FLAC, null-tests exports, and
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
| `render` | Offline-mix a project to WAV or FLAC with tags and dither |
| `render-stems` | Render each track to its own WAV or FLAC stem |
| `play` | Play a project on the default output device |
| `devices` | List output and input devices |
| `record` | Record an input device to a crash-safe WAV |
| `recover` | Finish a capture left behind by a forced kill |
| `delete-range` | Delete a range on the first track into autosave only |
| `autosave` | Write a recovery copy without replacing `project.json` |
| `recover-project` | Restore the latest autosave over `project.json` |
| `null-test` | Compare an export with a reference render |

`--json` is available on `decode`, `peaks`, `info`, `devices`, `recover`,
`recover-project` and `null-test`. `--log` sets the tracing filter.

## Data flow

`cli.rs` parses arguments with clap. `commands::dispatch` routes to
`commands::media`, `commands::project` or `commands::device`. Project commands
load through `ProjectPaths`, decode sources through `alp-media`, build a
`MixScene` and call `alp-engine`. Import converts non-project sample rates to
WAV first, then applies `AddSource` and `AddClip` through a `CommandStack` and
saves atomically. `delete-range` applies `DeleteRange` and writes autosave only,
which is how the Phase 3 harness models an edit that survives a forced kill.

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
render. `scripts/phase3-demo.sh` covers import conversion, autosave recovery,
WAV/FLAC export and null tests; `--record` and `--soak` add the hardware paths.

## Known limits

- `play` uses the default output; `record` accepts one named input.
- `record` writes float32 WAV and does not resample.
- `render` holds the whole mix in memory.
- `delete-range` is the only headless edit command; the rest of the edit set is
  exercised through `alp-core` and the desktop inspector.
- Import decodes synchronously.
