# `alp-media::decode`

## Purpose

Wraps symphonia. `probe` reads container parameters without decoding;
`decode_path` streams interleaved f32 chunks to a callback; `decode` is the
whole-file convenience wrapper used by the CLI and engine.

## Public API

- `MediaInfo { sample_rate, channels, frames: Option<u64>, codec }`
- `probe(path)`
- `decode_path(path, on_chunk)`
- `decode(path) -> PcmBuffer`

## Data flow

The file is opened with a `MediaSourceStream` and a format hint. The default
audio track is selected; packets are decoded by symphonia and converted to f32,
preserving the source channel count. `decode_path` invokes the callback per
packet and counts frames. `decode` accumulates the chunks and constructs a
`PcmBuffer`.

## Allowed dependencies

`symphonia` and other `alp-media` modules. No ALP crate except `alp-core` via
errors.

## Threading / real-time notes

Blocking, allocating and CPU-heavy. Always run on a worker. The decoded
`PcmBuffer` is plain owned data and can be moved to the engine as a snapshot.

## Error handling

Unreadable files are `Io`; unknown containers/codecs are `Unsupported`; packet
or parameter failures are `Decode`. A missing audio track is a decode error.
`decode` rejects an empty stream through `PcmBuffer::new`.

## Testing

One test decodes a generated stereo WAV and checks rate, channels, duration
and peak. Corrupt input and unsupported extensions are handled in code but not
yet unit-tested; FLAC/MP3/OGG depend on the enabled codec features.

## Known limits

- Whole-file decoding holds the entire track in memory.
- No gapless metadata, chapter or multi-track selection.
- `codec` is a debug name, not a stable user-facing label.
