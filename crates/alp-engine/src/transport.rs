//! Transport state machine.
//!
//! The transport is plain state; the device callback and the offline renderer
//! both advance it. No locks are required because the UI sends commands over the
//! lock-free channel in [`crate::command`].

use alp_core::FrameRange;

/// Playback state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportState {
    /// Not playing and at the start or an arbitrary seek position.
    Stopped,
    /// Advancing.
    Playing,
    /// Holding position.
    Paused,
}

/// Timeline position and loop range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transport {
    state: TransportState,
    position: u64,
    total_frames: u64,
    loop_range: Option<FrameRange>,
}

impl Transport {
    /// Create a stopped transport for a project of `total_frames`.
    #[must_use]
    pub fn new(total_frames: u64) -> Self {
        Self {
            state: TransportState::Stopped,
            position: 0,
            total_frames,
            loop_range: None,
        }
    }

    /// Current state.
    #[must_use]
    pub fn state(&self) -> TransportState {
        self.state
    }

    /// Current position in frames.
    #[must_use]
    pub fn position(&self) -> u64 {
        self.position
    }

    /// Project length in frames.
    #[must_use]
    pub fn total_frames(&self) -> u64 {
        self.total_frames
    }

    /// Whether the transport is advancing.
    #[must_use]
    pub fn is_playing(&self) -> bool {
        self.state == TransportState::Playing
    }

    /// Start playing from the current position.
    pub fn play(&mut self) {
        if self.position >= self.total_frames {
            self.position = 0;
        }
        self.state = TransportState::Playing;
    }

    /// Pause without moving.
    pub fn pause(&mut self) {
        if self.state == TransportState::Playing {
            self.state = TransportState::Paused;
        }
    }

    /// Stop and return to the start.
    pub fn stop(&mut self) {
        self.state = TransportState::Stopped;
        self.position = 0;
    }

    /// Seek to a frame, clamped to the project length.
    pub fn seek(&mut self, frame: u64) {
        self.position = frame.min(self.total_frames);
    }

    /// Set or clear the loop range.
    pub fn set_loop(&mut self, range: Option<FrameRange>) {
        self.loop_range = range.filter(|range| !range.is_empty());
    }

    /// Current loop range.
    #[must_use]
    pub fn loop_range(&self) -> Option<FrameRange> {
        self.loop_range
    }

    /// Advance by `frames` and return the new position.
    pub fn advance(&mut self, frames: u64) -> u64 {
        if self.state != TransportState::Playing || frames == 0 {
            return self.position;
        }
        if let Some(range) = self.loop_range {
            if self.position < range.start {
                self.position = range.start;
            }
            let next = self.position.saturating_add(frames);
            self.position = if next >= range.end {
                let span = range.len();
                range.start + (next - range.start) % span
            } else {
                next
            };
            return self.position;
        }

        self.position = self.position.saturating_add(frames).min(self.total_frames);
        if self.position >= self.total_frames {
            self.state = TransportState::Stopped;
        }
        self.position
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn play_pause_and_stop() {
        let mut transport = Transport::new(1_000);
        transport.play();
        assert_eq!(transport.advance(100), 100);
        transport.pause();
        assert_eq!(transport.advance(100), 100);
        transport.play();
        assert_eq!(transport.advance(50), 150);
        transport.stop();
        assert_eq!(transport.position(), 0);
        assert_eq!(transport.state(), TransportState::Stopped);
    }

    #[test]
    fn reaches_the_end_and_stops() {
        let mut transport = Transport::new(200);
        transport.play();
        transport.advance(250);
        assert_eq!(transport.position(), 200);
        assert_eq!(transport.state(), TransportState::Stopped);
    }

    #[test]
    fn loops_within_the_range() {
        let mut transport = Transport::new(1_000);
        transport.set_loop(Some(FrameRange::new(100, 200).expect("range")));
        transport.seek(150);
        transport.play();
        assert_eq!(transport.advance(30), 180);
        assert_eq!(transport.advance(50), 130);
    }

    #[test]
    fn seek_is_clamped() {
        let mut transport = Transport::new(100);
        transport.seek(500);
        assert_eq!(transport.position(), 100);
    }
}
