# `alp-ui` / `app/types`

## Purpose

Internal model structures and view data types utilized by the desktop application
shell, including drag state, audio effects representation, speech generation history,
and capture items.

## Public API

- `Drag { Panel, Resize }`
- `EffectItem { name, enabled, params }`
- `HistoryItem { text, voice_name, path, duration_secs, created, kind }`
- `CaptureItem { name, path, duration_secs, transcript, peak }`
- `GenerationKind { Take, Story }`
- `GeneratedTake { text, voice_name, path, duration_secs, kind }`
- `RenderOutput { Take, Story }`

## Data flow

These types are owned by `App` state and populated during user interactions,
background audio synthesis jobs, and capture recording sessions. Views read
these items to populate lists, cards, and history feeds.

## Allowed dependencies

- `alp-media`: `PcmBuffer`
- `iced`: `Point`
- `crate::panels::PanelKind`
- `std::path::PathBuf`

## Threading / real-time notes

Allocated and mutated strictly on the UI thread or delivered from background
worker completions. No interaction with the real-time audio thread callback.

## Error handling

Optional paths and duration calculations default gracefully; malformed items
are skipped or filtered during view rendering.

## Testing

Covered by studio tests in `app::studio::tests` and view generation checks.

## Known limits

- In-memory history cache size bounded by application memory.
