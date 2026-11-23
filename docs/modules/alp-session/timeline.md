# `alp-session::timeline`

## Purpose

Data transfer representations and serialization structures for the multitrack timeline editor. Connects headless `EditSession` command state with the Tauri IPC bridge and webview Canvas 2D renderer.

## Public API

- `TimelineDto` — Authoritative project timeline state (tracks, markers, loop range, active selection, undo/redo availability).
- `TimelineTrackDto` — Serialized track representation (gain, pan, mute, solo, arm, clips).
- `TimelineClipDto` — Serialized non-destructive audio clip (start, duration, gain, fades, envelope).
- `TimelineEnvelopePointDto` — Automation curve control point (offset seconds, linear gain).
- `TimelineMarkerDto` — Cue marker annotation point (timestamp seconds, label).
- `TimelineLoopRangeDto` — Bounded loop playback range (start seconds, end seconds).
- `TimelineSelectionDto` — Bounded in-out timeline editing selection (track id, start seconds, end seconds).

## Data flow

1. The React multitrack canvas requests timeline state through `get_timeline_state` IPC command.
2. `EditSession::timeline_dto()` translates internal sample frames into double-precision seconds using `sample_rate`.
3. Timeline editing operations (`move_clip_by_id`, `trim_clip_by_id`, `set_clip_fades_by_id`, `set_clip_envelope_by_id`) translate UI seconds back into exact frame offsets and dispatch undoable `alp_core::Command` objects.
4. Each successful edit increments `session.revision` and emits `session_revision_changed`.

## Allowed dependencies

- `serde` (Serialize, Deserialize)
- `alp-core` (Command types, time conversion, identifier primitives)

## Threading / real-time notes

Serialization executes on the calling IPC worker thread under `AppState::session` lock acquisition. No audio real-time threads are blocked.

## Error handling

Invalid clip IDs or out-of-bounds frame parameters return descriptive `Result<T, String>` errors without mutating project state. Failed commands leave the command stack untouched.

## Testing

Unit tests in `session::tests::timeline_operations_undo_redo_and_clip_id_preservation` verify:
- Timeline serialization round-trips.
- Split clip command idempotency and clip ID preservation across undo and redo cycles.
- Non-destructive move, trim, fade, and envelope adjustments.

## Known limits

Real-time audio waveform peak pyramid rendering uses separate binary channel buffers (`get_waveform_peaks`) to avoid JSON serialization overhead.
