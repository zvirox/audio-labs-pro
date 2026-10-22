# `alp-ui`

## Purpose

The desktop application shell: one transparent window, floating panels, input
handling and the glass surface. Phase 1 keeps the binary as a placeholder so the
crate graph and dependency guard are real from day one.

## Public API

The crate is a binary with no library API. Phase 1 `main` prints a notice and
exits successfully so the workspace builds on every platform.

## Data flow

Planned: the UI owns screens and the command stack, reads project state from
`alp-core`, decodes through `alp-media`, controls `alp-engine` and calls voice
backends through `alp-voice`. It never calls cpal, ONNX or GGUF directly.

## Allowed dependencies

`alp-core`, `alp-media`, `alp-engine`, `alp-voice` and `alp-ui-kit`. It must not
depend on `alp-infer`, `alp-dsp` implementation details or cpal.

## Threading / real-time notes

All UI work runs on the main/UI thread. Decoding, render and inference are sent
to workers or the inference process. The UI never blocks the audio callback and
never calls it directly; it sends commands over the engine channel.

## Error handling

Phase 1 exits successfully after a notice. The Phase 2 shell will surface
recoverable errors in the UI and keep the window alive.

## Testing

CI builds the binary. UI tests and headless snapshot tests arrive in Phase 2.

## Known limits

- No window, panels, glass or input handling yet.
- No accessibility tree or reduce-motion integration yet.
