# `alp-cli::commands`

## Purpose

Maps the parsed `Command` enum to the media, project or device command
functions. It is a thin, exhaustive dispatcher.

## Public API

- `pub mod media`, `pub mod project`, `pub mod device`
- `dispatch(command: Command) -> anyhow::Result<()>`

## Data flow

`dispatch` destructures each variant and calls the matching function, passing
paths and options by reference where appropriate. Errors propagate unchanged to
`main`.

## Allowed dependencies

`anyhow`, `crate::cli::Command`, and the three command modules. Those in turn use
`alp-core`, `alp-media` and `alp-engine`.

## Threading / real-time notes

Synchronous on the main thread. The device calls may block for the duration of
playback or recording.

## Error handling

There is no error wrapping here; each command supplies its own context. A new
`Command` variant forces a new match arm at compile time.

## Testing

Exercised by `scripts/phase1-demo.sh` and `cargo test --workspace` (the library
builds all arms).

## Known limits

- One command per process invocation; no batch mode or scripting REPL.
- Exit codes are limited to success (0) and failure (1).
