//! Lock-free command channel between UI/workers and the audio callback.
//!
//! The producer side allocates nothing after construction; the consumer never
//! blocks. Commands are small `Copy` values so they can live in the ring buffer.

use alp_core::FrameRange;
use rtrb::{Consumer, Producer, PushError, RingBuffer};

/// A control message for the audio thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineCommand {
    /// Start playing from the current position.
    Play,
    /// Pause at the current position.
    Pause,
    /// Stop and return to the start.
    Stop,
    /// Seek to an absolute frame.
    Seek(u64),
    /// Set or clear the loop range.
    SetLoop(Option<FrameRange>),
    /// Ask the callback to exit after the next buffer.
    Shutdown,
}

/// Sending half of the command channel.
#[derive(Debug)]
pub struct CommandSender {
    producer: Producer<EngineCommand>,
}

/// Receiving half of the command channel.
#[derive(Debug)]
pub struct CommandReceiver {
    consumer: Consumer<EngineCommand>,
}

/// Create a bounded command channel with `capacity` slots.
#[must_use]
pub fn command_channel(capacity: usize) -> (CommandSender, CommandReceiver) {
    let (producer, consumer) = RingBuffer::new(capacity.max(1));
    (CommandSender { producer }, CommandReceiver { consumer })
}

impl CommandSender {
    /// Try to enqueue a command without blocking.
    ///
    /// `Err(command)` means the queue is full; the caller can retry or drop the
    /// message. A full queue is a programming error in normal UI use.
    pub fn try_send(&mut self, command: EngineCommand) -> Result<(), EngineCommand> {
        self.producer.push(command).map_err(|error| match error {
            PushError::Full(command) => command,
        })
    }

    /// Number of free slots.
    #[must_use]
    pub fn free_slots(&self) -> usize {
        self.producer.slots()
    }
}

impl CommandReceiver {
    /// Take the next command, if any.
    pub fn try_recv(&mut self) -> Option<EngineCommand> {
        self.consumer.pop().ok()
    }

    /// Number of queued commands.
    #[must_use]
    pub fn len(&self) -> usize {
        self.consumer.slots()
    }

    /// Whether no command is queued.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.consumer.slots() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_round_trip_in_order() {
        let (mut sender, mut receiver) = command_channel(4);
        sender.try_send(EngineCommand::Play).expect("send");
        sender.try_send(EngineCommand::Seek(480)).expect("send");
        assert_eq!(receiver.try_recv(), Some(EngineCommand::Play));
        assert_eq!(receiver.try_recv(), Some(EngineCommand::Seek(480)));
        assert_eq!(receiver.try_recv(), None);
    }

    #[test]
    fn full_queue_returns_the_command() {
        let (mut sender, _receiver) = command_channel(1);
        sender.try_send(EngineCommand::Play).expect("first");
        assert_eq!(
            sender.try_send(EngineCommand::Pause),
            Err(EngineCommand::Pause)
        );
    }
}
