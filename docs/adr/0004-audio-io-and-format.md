# 0004 — Internal audio format and real-time I/O (D4)

- Status: accepted
- Date: 2026-10-22
- Spike: `spikes/audio-spike`

## Context

The engine must play many clips sample-accurately and record without dropouts,
while the UI stays responsive. cpal is the only realistic Linux-first I/O crate
in Rust and its API changed in 0.18 (typed `Device` display, `SampleRate = u32`,
`build_*_stream(config, callback, err, timeout)`).

## Decision

- Internal samples: **f32**, interleaved, de-interleaved per bus as needed.
- Default project rate: **48 kHz**, per-project override. Sources resample to the
  project rate with `rubato` (pinned).
- Engine commands reach the audio callback over an `rtrb` SPSC ring buffer;
  audio data reaches the callback from pre-decoded buffers only.
- The callback only copies/mixes and writes to the device buffer; all decoding,
  resampling and peak work happens in workers.
- cpal 0.18, ALSA host; PipeWire is reached through its ALSA/Pulse compatibility
  layer (verified: default device 48 kHz, 2 ch, f32).

## Evidence

- Playback: 956 callbacks over 10 s, mean interval 10.66 ms, worst 11.13 ms,
  zero interval gaps; shortfall limited to the deliberate tail (≈113 ms).
- Capture: 467 callbacks over 5 s, mean 10.67 ms, worst 12.35 ms, zero gaps,
  WAV written successfully.
- 1-hour WAV: decode + peaks in 998 ms; cache 3.38 MB; reload 0.92 ms.

## Consequences

- No lock, allocation, logging or syscall in the callback; enforced in Phase 1 by
  a debug allocator guard.
- Device format negotiation must support F32 and I16 at least (both seen).
- Latency is reported as the callback period (≈10.7 ms at 48 kHz / 512 frames);
  the app must expose buffer-size settings for slower machines.
