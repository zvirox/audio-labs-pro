# `alp-cli::cli`

## Purpose

Clap grammar for every Phase 1 command. This module only describes arguments;
it performs no I/O.

## Public API

- `Cli { log, command }`
- `Command` variants: `Gen`, `Decode`, `Peaks`, `New`, `Info`, `Import`,
  `Render`, `Play`, `Devices`, `Record`

## Data flow

Clap parses `std::env::args` into `Cli`. Global `--log` is accepted before or
after the subcommand. Paths use `PathBuf`. Numeric options have sensible
defaults (48 kHz, 3,600 s for `gen`, 5 s for `record`, `int16` for render).

## Allowed dependencies

`clap` with the derive feature, `std::path`. It does not call `alp-*` crates.

## Threading / real-time notes

Parsing happens once at startup on the main thread. No audio-thread interaction.

## Error handling

Clap prints usage and exits with code 2 for invalid arguments. `--help` and
`--version` exit successfully.

## Testing

Covered indirectly by the end-to-end demo script. There are no unit tests for
the grammar; clap itself is trusted for parsing.

## Known limits

- No shell completion generation yet.
- No config-file defaults; every option is a flag.
- `play` and `record` cannot select a non-default device.
