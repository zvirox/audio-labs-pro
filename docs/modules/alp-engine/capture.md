# `alp-engine::capture`

## Purpose

Records the default input device to a float32 WAV file without doing disk I/O
in the audio callback.

## Public API

- `record_to_wav(path, seconds) -> Result<CaptureReport, EngineError>`
- `CaptureReport { frames, sample_rate, channels, dropped, callbacks,
  elapsed_secs, stream_error }`

## Data flow

The input callback converts f32/i16 samples and pushes them into an `rtrb` SPSC
ring; a full ring increments a dropped-sample counter. A writer thread drains
the ring into a hound float WAV temp file. After the requested wall-clock
duration the stream is dropped, the writer is told to finish, and the temp file
is atomically renamed over the destination.

## Allowed dependencies

`alp-core` (temp files/commit), `alp-media` (errors), `cpal`, `rtrb`, `hound`,
`std::thread`, `thiserror`.

## Threading / real-time notes

- Callback: conversion and `Producer::push` only; no allocation or locks.
- Writer thread: WAV encoding and disk I/O.
- Caller thread: sleeps for the duration, then joins.
- The ring capacity is two seconds (minimum 65,536 samples).

## Error handling

No device or unsupported format fails before the stream starts. A build/start
failure stops the writer and removes the temp file. Writer errors are returned
after the stream stops. `dropped > 0` or `stream_error` indicates the capture is
not trustworthy; the CLI reports both.

## Testing

No CI test opens a capture device. The shared i16 conversion helper is unit
tested; the capture ring, writer thread and device path are acceptance checks in
`scripts/phase1-demo.sh --record`.

## Known limits

- Default input device only.
- No resampling or channel selection.
- Float32 WAV only.
- A slow writer can drop samples; the counter makes this visible rather than
  hiding it.
