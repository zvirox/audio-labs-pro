# `alp-media` / `export`

## Purpose

One export path for WAV and FLAC with resampling, bit-depth selection, dither,
tags and atomic replacement.

## Public API

- `ExportFormat::{Wav, Flac}`
- `AudioMetadata { title, artist, album, comment }`
- `ExportSettings { format, bit_depth, sample_rate, dither, metadata }`
- `ExportReport { frames, sample_rate, channels, bits_per_sample, format, peak, rms }`
- `export(path, buffer, settings)`

## Data flow

`export` resamples to the requested rate, then dispatches to the WAV writer or
the FLAC writer. WAV writes RIFF `INFO` tags; FLAC writes a Vorbis comment block.
Both paths write a sibling temporary file and rename it atomically.

## Allowed dependencies

`alp-core` for atomic files; `flacenc`, `hound` and `rubato` for encoding.

## Threading / real-time notes

Offline worker operation. It allocates a converted buffer and an integer sample
vector, so it must never run on the audio callback.

## Error handling

Float FLAC, unknown containers, encode failures and I/O errors are returned as
`MediaError`. A failed write leaves the previous target untouched.

## Testing

Tests cover format parsing, resampling plus RIFF tags, FLAC round-trip through
Symphonia, and float-FLAC rejection.

## Known limits

- FLAC metadata is emitted as a type-4 block through `MetadataBlockData`.
- Symphonia 0.6.1 rejects `flacenc` streams whose STREAMINFO minimum block size
  differs from the maximum; the writer sets both to the encoder block size.
  `flac` and `metaflac` accept the result, and so does Symphonia after the
  adjustment.
- OGG/MP3 export remains Phase 5 work.
