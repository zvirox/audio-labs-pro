# `alp-core::logging`

## Purpose

Initialises the process-wide tracing subscriber once, with a filter from the
`ALP_LOG` environment variable or an explicit CLI argument.

## Public API

- `DEFAULT_FILTER` (`"info"`)
- `init(filter: Option<&str>) -> Result<(), CoreError>`

## Data flow

The CLI calls `init(cli.log.as_deref())` before dispatch. `init` reads the
filter, builds a `tracing_subscriber` fmt subscriber with ANSI output, and
installs it. Calling it twice returns an error rather than panicking.

## Allowed dependencies

`tracing`, `tracing-subscriber`, `CoreError`. No other ALP crate.

## Threading / real-time notes

Call once at startup on the main thread. Logging from the audio callback is
forbidden by the engine rules; the subscriber itself is not real-time safe.

## Error handling

A bad filter string or an already-installed subscriber returns
`CoreError::Logging`. The CLI treats this as fatal.

## Testing

No dedicated test; the CLI initialises tracing once at startup and the
workspace tests leave it uninitialised.

## Known limits

- One global subscriber; no per-session log file.
- ANSI output only; JSON logs are not implemented.
