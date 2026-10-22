# `alp-engine::command`

## Purpose

A bounded lock-free SPSC channel for control messages sent from the UI or a
worker to the audio callback. It replaces locks and shared mutable state.

## Public API

- `EngineCommand::{Play, Pause, Stop, Seek(u64), SetLoop(Option<FrameRange>),
  Shutdown}`
- `command_channel(capacity) -> (CommandSender, CommandReceiver)`
- `CommandSender::try_send`, `free_slots`
- `CommandReceiver::try_recv`, `len`, `is_empty`

## Data flow

The producer pushes small `Copy` commands into an `rtrb` ring; the consumer
drains them at the start of a callback. A full ring returns the rejected command
instead of blocking. `Shutdown` asks the callback to exit after the next buffer.

## Allowed dependencies

`rtrb`, `alp_core::FrameRange`, `EngineError`. No other ALP crate.

## Threading / real-time notes

`try_send` and `try_recv` are wait-free and allocation-free after channel
construction. Capacity must be chosen before the stream starts. There is exactly
one producer and one consumer.

## Error handling

`try_send` returns the command when the ring is full so the caller can retry,
coalesce or report backpressure. `try_recv` returns `None` when empty.

## Testing

Tests send/receive in order and verify a full queue returns the original
command without losing earlier entries.

## Known limits

- Single producer and single consumer only.
- No priority channel; state changes are processed in FIFO order.
- Capacity is not resizable.
