# Phase 4 results — multitrack timeline and mixing

Status: implementation in progress; backend and editor commands are green. Live cross-track drag and hardware multitrack capture still remain.

## Automated/headless checks

`bash scripts/phase4-demo.sh` passed on this machine:

| Check | Result |
|---|---|
| project tracks | 2 (one edited) |
| track gain/pan | 0.75 / +0.25 · master_gain 0.85 |
| render null check | max diff 0.00000000 |
| stem export | 1 non-empty track, 144,000 frames |
| workspace tests | 154 passing |
| fmt / clippy | green |

Engine test `renders_ten_tracks` processes a 10-track scene with no failure.
`render-stems` exports each track independently as WAV or FLAC.

## Live desktop checks

- `ALP_AUTOCLOSE_SECS=3 ALP_FORCE_TINT=1 cargo run -p alp-ui` exits cleanly.
- Audio playback now uses `play_scene_live`, feeding transport position, peak, RMS and per-track meters.

## Known limits

- Mixer meters are driven from playback and render, but no multi-input capture exists yet.
- Dragging clips between tracks is not pointer-based; `MoveClip`/`MoveClip to next/previous track` commands expose track movement.
- Ripple is selected through a toggle; the full inspector affordances are still minimal.
- Crossfade command is implemented and tested, but its inspector UI is a single 1.0 s trigger.
- OGG/MP3 export remains blocked by encoder and licence verification.
