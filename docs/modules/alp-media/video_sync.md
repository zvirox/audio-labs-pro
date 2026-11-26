# `alp-media` / `video_sync`

## Purpose

Video demux/remux support for dubbing. Extracts a video's audio track to PCM, and remuxes a dubbed audio track back into MP4/MKV while stream-copying the video.

## Public API

- `RemuxRequest { video_path, audio_path, output_path, subtitle_path }`.
- `RemuxReport { output_path, duration_secs, video_stream_copied, audio_codec }`.
- `VideoProbe { duration_secs, format_name }`.
- `VideoRemuxer::{new, is_available, probe, extract_audio, remux}`.

## Data flow

1. `probe` calls `ffprobe` for container duration and format.
2. `extract_audio` calls `ffmpeg -vn -ac 2 -ar 48000 -c:a pcm_s16le` to a sibling temp WAV, decodes it with `alp_media::decode`, then removes the temp file.
3. `remux` builds an explicit ffmpeg argument vector with `-map 0:v:0 -map 1:a:0 -c:v copy -c:a aac -shortest`.
4. Output is written to a `.part-<pid>` sibling and renamed into place; optional subtitles are copied to a side `.srt`.

## Allowed dependencies

`alp-core` (atomic writes), `serde` for DTOs, and the external `ffmpeg`/`ffprobe` binaries. No Rust ffmpeg crate and no new internal dependency.

## Threading / real-time notes

Spawns external processes; synchronous and CPU/IO bound. Call from a job worker, never from the audio callback. `is_available` performs a cheap `-version` probe.

## Error handling

Missing binaries, non-zero exit codes, and invalid media return `MediaError::Decode` (probe/extract) or `MediaError::Encode` (remux). Temporary files are removed on failure. The output file is only renamed after ffmpeg succeeds.

## Testing

Tests verify that the argument vector preserves `-c:v copy` and the correct audio map, and that `is_available` is stable. Runtime remux tests require FFmpeg and are intentionally not mandatory for CI.

## Known limits

- Requires `ffmpeg` and `ffprobe` on PATH (or `ALP_FFMPEG` / `ALP_FFPROBE` overrides).
- Video is always stream-copied; re-encoding video is out of scope.
- Subtitle paths are copied as side files, never burned into the video.
- FFmpeg licensing depends on the installed build; the app does not redistribute it.
