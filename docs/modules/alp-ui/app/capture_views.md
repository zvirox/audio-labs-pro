# `alp-ui` / `app/capture_views`

## Purpose

Renders the Captures workspace: recording list, record/punch/monitor controls,
live input meters, the transcript segment editor, and transcript export actions.

## Public API

- `App::captures_workspace(&self, colors: ThemeColors) -> Element<'_, Message>`

## Data flow

1. Reads `captures`, `selected_capture`, `transcript`, `selected_segment`,
   `recording`, `record_peak`/`record_rms`, `input_devices` and `dictating`.
2. Emits `ToggleRecord`, `TogglePunchIn`, `ToggleMonitor`, `SelectCapture`,
   `TranscribeActiveTrack`, `ToggleDictation`, `SelectTranscriptSegment`,
   `DeleteTranscriptSegment`, `ExportTranscriptSrt` and `ExportTranscriptTxt`.
3. Recording and transcription are performed by `alp-engine` and
   `alp-voice::BuiltinSttBackend` through handlers in `studio.rs`/`update.rs`.

## Allowed dependencies

- `iced::widget` primitives, `alp-ui-kit` components and tokens
- `super::{App, Message, format_time}`, `super::workspace_layout::{card, header}`

## Threading / real-time notes

Meters are sampled from the engine's reporter and displayed once per frame.
Transcription decodes the selected file on the UI thread; captures are short,
but this is the one synchronous decode left in the workspace.

## Error handling

Missing captures show an empty state. Decode and write failures become toasts.
The transcript empty state explains that segmentation needs a capture first.

## Testing

Compile-checked and exercised by the smoke run; the VAD path is covered by
`alp-voice` unit tests.

## Known limits

- The built-in STT only detects speech regions and emits placeholder words.
- Capture rows are session-only; they are not reloaded from disk on boot.
