# `alp-media`

## Purpose

Turns files into audio and audio into files. It owns symphonia decoding,
rubato resampling, the interleaved f32 buffer type, the three-level peak
pyramid and its on-disk cache, deterministic synthetic fixtures, and WAV output.
The UI and engine never parse media containers themselves.

## Public API

- `decode`, `decode_path`, `probe`, `MediaInfo`
- `PcmBuffer` with `new`, `silence`, `samples`, `samples_mut`, `frames`,
  `duration_secs`, `peak`, `rms`
- `resample`
- `PeakBuilder`, `PeakPyramid`, `PeakLevel`, `PeakCache`, `CacheStatus`
- `write_wav`, `WavBitDepth` (`Int16`, `Int24`, `Float32`)
- `ExportFormat`, `AudioMetadata`, `ExportSettings`, `ExportReport`, `export`
- `DitherMode`, `Quantizer` (internal)
- `NullReport`, `compare`
- `write_test_tone`
- Constants `BASE_BUCKET`, `LEVEL_COUNT`, `CACHE_MAGIC`, `CACHE_VERSION`,
  `PEAKS_DIR`
- `MediaError`

## Data flow

1. `probe` opens a file with symphonia and returns rate, channels, codec and an
   optional frame count.
2. `decode` selects the default audio track, converts every packet to f32 and
   interleaves channels into a `PcmBuffer`.
3. `resample` converts a buffer to a target rate with rubato’s FFT resampler.
4. `PeakBuilder` consumes decoded frames in chunks and produces levels at 256,
   1,024 and 4,096 frames per bucket.
5. `PeakCache::load_or_build` validates a cache header, sample count and bucket
   geometry; on a miss it decodes, builds and atomically commits the cache.
6. `export` resamples, quantizes with optional TPDF dither, writes WAV or FLAC
   with metadata and renames a sibling temp file into place.
7. `compare` subtracts an export from a reference for a null test.

## Allowed dependencies

`alp-core` only. External crates are `symphonia`, `rubato`, `hound`, `thiserror`
and `tracing`.

## Threading / real-time notes

Decoding, resampling, peak building and file I/O are worker-thread work. They
allocate and block and must never run on the audio callback. `PcmBuffer`,
`PeakPyramid` and cache values are immutable snapshots once handed to the
engine or UI.

## Error handling

`MediaError` separates I/O, unsupported codec/format, decode, resample, encode
and invalid-cache failures. A corrupt or stale cache is treated as a miss and
rebuilt; the cache writer always finishes with an atomic rename, so a crash
leaves either the old cache or no cache.

## Testing

Tests cover PCM dimension validation and peak/RMS, WAV header writing at
16-bit and float32 plus bit-depth parsing, resampling identity and duration,
synthetic tone generation and argument rejection, peak-pyramid geometry and
level selection, a peak-cache hit plus corrupt-cache rebuild, dither, WAV tags,
a FLAC export round-trip through Symphonia, and null-test comparison. Decode is
exercised by generated WAV and FLAC files.

## Known limits

- Encoding is WAV and FLAC; MP3/OGG export arrives in a later phase.
- FLAC STREAMINFO min/max block sizes are both set to the encoder block size to
  work around a Symphonia 0.6.1 short-final-block rejection.
- Resampling is offline only; the engine resamples a prepared scene before
  playback rather than streaming.
- Media is decoded fully into memory. A one-hour stereo float buffer is about
  1.3 GiB; long-file streaming is a later optimisation.
