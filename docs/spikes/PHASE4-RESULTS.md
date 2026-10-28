# Phase 4 results — multitrack timeline and mixing

Status: multitrack timeline, mixing, stems, crossfades, delivery presets, and markers completed.

## Automated/headless checks

`bash scripts/phase4-demo.sh` passed on this machine:

| Check | Result |
|---|---|
| project tracks | 2 (one edited) |
| track gain/pan | 0.75 / +0.25 · master_gain 0.85 |
| render null check | max diff 0.00000000 |
| stem export | 1 non-empty track, 144,000 frames |
| delivery presets | Podcast (FLAC), Voice-Over (WAV) generated and verified |
| workspace tests | 158 passing |
| fmt / clippy | green (`-D warnings` enforced) |
| guard scripts | file-length (≤1,500 loc), check-deps, check-docs (75 docs), check-ui |

Engine test `renders_ten_tracks` processes a 10-track scene with no failure.
`render-stems` exports each track independently as WAV or FLAC.

## Live desktop checks

- `ALP_AUTOCLOSE_SECS=3 ALP_FORCE_TINT=1 cargo run -p alp-ui` exits cleanly.
- Audio playback now uses `play_scene_live`, feeding transport position, peak, RMS and per-track meters.
- Inspector provides selectable crossfade intervals (0.5s, 1.0s) and track shifts.
- Export Queue supports direct preset buttons: Podcast, Voice-Over, Audiobook.
- Timeline dynamically renders project tracks, markers, and loop ranges.

## Known limits

- Mixer meters are driven from playback and render, but multi-hardware-interface capture is planned for after Phase 5.
- Dragging clips across tracks uses inspector buttons and keyboard commands rather than pointer drag-and-drop.
- OGG/MP3 export remains blocked by encoder and license verification per ADR 0005.
