# `alp-ui` / `app/message`

## Purpose

Defines the top-level `Message` enum and `CreateVoiceTab` modal state that drive
all interactive user actions, asynchronous job updates, and window events across
the Audio Labs Pro desktop shell.

## Public API

- `CreateVoiceTab { Upload, Record, SystemAudio }` with `index(&self) -> usize`
- `Message` enum containing all shell message variants

## Data flow

UI views, subscriptions, and worker queues construct `Message` instances and emit
them into the iced runtime. The central `App::update` method pattern-matches each
message, updating application state, invoking `EditSession` commands, or triggering
background tasks.

## Allowed dependencies

- `alp-core`: `ClipId`
- `alp-media`: `ExportPreset`
- `alp-api`: `control::{ApprovalDecision, ControlEvent}`, `PermissionTier`
- `iced`: `Point`, `Size`, `window`, `widget::text_editor`
- `crate::commands::CommandId`
- `crate::glass::GlassReport`
- `crate::panels::PanelKind`
- `crate::app::workspace_layout::WorkflowTab`

## Threading / real-time notes

Messages are lightweight data structures passed strictly across the UI event loop.
Worker threads communicate results back to the UI via channels wrapped into `Message`
subscriptions. They never touch the real-time audio processing thread.

## Error handling

Operations that can fail (e.g., file import, synthesis, model downloads) carry structured
result payloads within message variants. Error variants trigger visual toasts, inline
warnings, or status updates rather than panicking.

## Testing

Covered by unit tests in `app::tests` verifying shortcut bindings and message-to-command
mappings, as well as UI end-to-end integration tests.

## Known limits

- Single monolithic event enum per iced architecture.
- Large payload data is referenced via paths or shared handles rather than copied directly.
