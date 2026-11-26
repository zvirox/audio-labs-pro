# `alp-engine` / `macro_chain`

## Purpose

Directory batch processing for DSP macro chains. Applies an ordered list of built-in effects to every matching audio file and writes WAV results.

## Public API

- `MacroStep { effect_id, params }` with `to_effect_ref(index)`.
- `MacroChain { steps }` with `build() -> EffectChain`.
- `MacroRequest { input_dir, output_dir, pattern, overwrite, steps }`.
- `MacroFileResult`, `MacroReport`.
- `process_directory(&MacroRequest, progress) -> Result<MacroReport, EngineError>`.

## Data flow

1. Enumerate audio files (wav/flac/mp3/ogg/aiff) in the input directory, optionally filtering by a filename substring, sorted by path.
2. Build one `EffectChain` from the request's steps via the insert-rack effect factory.
3. For each file: decode to PCM, reset/prepare the chain, process in place, export 24-bit WAV to the output directory.
4. Report per-file outcomes, counts, and cooperative cancellation.

## Allowed dependencies

`alp-core` (`EffectRef`), `alp-dsp` (`EffectChain`), `alp-media` (decode/export/PcmBuffer), and `serde` for DTOs. All are existing `alp-engine` dependencies.

## Threading / real-time notes

Fully offline. Run from a worker/job thread; never call from the audio callback. The progress callback is invoked on the worker thread.

## Error handling

Directory/file I/O failures and invalid media are reflected in `MacroReport.failed` per file, not as whole-batch errors; only unreadable input directories return `EngineError`. Existing outputs are skipped unless `overwrite` is set.

## Testing

Tests verify two-file batch processing, substring filtering, and cooperative cancellation via the progress callback.

## Known limits

- Output is always WAV (24-bit); the input container is not preserved.
- `pattern` is a substring match, not a glob expression.
- Unknown effect ids fall back to unity gain, matching live insert behaviour.
