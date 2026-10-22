# `alp-engine::error`

## Purpose

One error type for scene validation, devices, streams, transport, media loading
and filesystem failures.

## Public API

`EngineError` variants: `NoDevice`, `Stream`, `UnsupportedFormat`,
`InvalidScene`, `Transport`, `Media(#[from] MediaError)`,
`Core(#[from] CoreError)`, `Io(#[from] std::io::Error)`.

## Data flow

Media and core errors convert automatically, so scene construction and render
code can use `?`. Device code maps cpal errors to `Stream` with context.

## Allowed dependencies

`alp-core`, `alp-media`, `thiserror`.

## Threading / real-time notes

Plain error values; formatting allocates. The audio callback never constructs
one. Runtime stream errors use an atomic flag instead.

## Error handling

`InvalidScene` names the clip or source problem. `NoDevice` means no hardware
was found, not a programming bug. `stream_error` in the reports covers failures
that happen after a stream starts.

## Testing

`error.rs` has no dedicated test module. Scene tests reach `InvalidScene` for
oversized ranges; media and core conversions are implemented but not asserted
directly.

## Known limits

- cpal error strings are not classified further.
- No retry/backoff policy for transient device errors.
