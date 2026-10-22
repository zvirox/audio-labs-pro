# `alp-ui` / `session/import`

## Purpose

Decodes a dropped audio file on a worker thread and hands the buffer back to the
editing session without blocking the UI thread.

## Public API

Adds `EditSession::is_importing`, `begin_import` and `poll_import`. The internal
`ImportedAudio` carrier stays private to the module.

## Data flow

`begin_import` clones the display name and path, spawns `alp-ui-import`, decodes
with `alp-media`, and sends `Result<ImportedAudio, String>` over an `mpsc`
channel. `poll_import` drains it non-blockingly from a frame tick and calls
`EditSession::import_buffer`, which copies the audio into project media and
issues `AddSource`/`AddClip`.

## Allowed dependencies

The parent `session` module, `alp-media::decode`, `std::thread` and `mpsc`.

## Threading / real-time notes

One import at a time. The worker only decodes; project mutation stays on the UI
thread. Frames are subscribed while an import is pending.

## Error handling

A decode/worker error becomes a session status line. A disconnected channel is
reported instead of hanging. Starting a second import returns an error.

## Testing

`session/tests.rs` decodes a generated WAV on the worker and polls until the clip
appears.

## Known limits

- No per-file progress or cancellation.
- Recording import uses the synchronous `import_file` path instead.
