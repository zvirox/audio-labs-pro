# `alp-ui` / `app/capture_views`

## Purpose

Renders the Captures workspace: the recordings list with punch-in/monitor
controls, the transcript segment editor, and the input card with device picker
and live peak/RMS meters.

## Public API

- `App::captures_workspace(&self, colors: ThemeColors) -> Element<'_, Message>`
- Private helpers `capture_list_view`, `transcript_view`, `input_view`,
  `recording_meters_view`.

## Data flow

1. Reads `captures`, `selected_capture`, `transcript`, `selected_segment`,
   `recording`, `record_peak`/`record_rms`, `input_devices`, `record_device`,
   `punch_in`, `record_monitor` and `dictating`.
2. Emits `ToggleRecord`, `TogglePunchIn`, `ToggleMonitor`, `SelectCapture`,
   `TranscribeActiveTrack`, `ToggleDictation`, `SelectTranscriptSegment`,
   `DeleteTranscriptSegment`, `ExportTranscriptSrt`, `ExportTranscriptTxt`,
   `SelectInput` and `SelectWorkflowTab`.
3. Recording and transcription are performed by `alp-engine` and
   `alp-voice::BuiltinSttBackend` through handlers in `studio.rs`/`update.rs`.
   Both lists scroll through `App::scroll`.

## Allowed dependencies

- `iced::widget` primitives, `alp-ui-kit` components and tokens
- `super::{App, Message, format_time}`,
  `super::workspace_layout::{card, card_auto, header}`

## Threading / real-time notes

Meters are sampled from the engine's reporter once per frame; each meter row uses
a fixed 150 px bar so the dBFS value stays aligned. Transcription decodes the
selected file on the UI thread; captures are short, but this is the one
synchronous decode left in the workspace.

## Error handling

Missing captures show an empty card. Decode and write failures become toasts.
The transcript empty state explains that segmentation needs a capture first and
that segment text is a placeholder.

## Testing

Compile-checked and exercised by the smoke run; the VAD path is covered by
`alp-voice` unit tests.

## Known limits

- The built-in STT only detects speech regions and emits placeholder words.
- Capture rows are session-only; they are not reloaded from disk on boot.
- The input meter shows the reporter's peak/RMS, not a per-channel histogram.
