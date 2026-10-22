//! Frame-based time types.
//!
//! All timeline positions in `alp-core` are expressed in sample frames at the
//! project sample rate. Seconds are only used at the presentation boundary.

use serde::{Deserialize, Serialize};

use crate::CoreError;

/// A half-open range of frames `[start, end)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FrameRange {
    /// First frame in the range.
    pub start: u64,
    /// One past the last frame in the range.
    pub end: u64,
}

impl FrameRange {
    /// Build a range, rejecting `end < start`.
    pub fn new(start: u64, end: u64) -> Result<Self, CoreError> {
        if end < start {
            return Err(CoreError::InvalidRange { start, end });
        }
        Ok(Self { start, end })
    }

    /// Number of frames covered by the range.
    #[must_use]
    pub fn len(self) -> u64 {
        self.end - self.start
    }

    /// Whether the range contains no frames.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Whether a frame lies inside the range.
    #[must_use]
    pub fn contains(self, frame: u64) -> bool {
        frame >= self.start && frame < self.end
    }
}

/// Convert frames to seconds at a sample rate.
#[must_use]
pub fn frames_to_secs(frames: u64, sample_rate: u32) -> f64 {
    if sample_rate == 0 {
        return 0.0;
    }
    frames as f64 / f64::from(sample_rate)
}

/// Convert seconds to the nearest frame count, saturating at zero.
#[must_use]
pub fn secs_to_frames(seconds: f64, sample_rate: u32) -> u64 {
    if !seconds.is_finite() || seconds <= 0.0 {
        return 0;
    }
    (seconds * f64::from(sample_rate)).round() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_rejects_inverted_bounds() {
        assert!(FrameRange::new(10, 9).is_err());
        assert_eq!(FrameRange::new(10, 10).expect("valid").len(), 0);
    }

    #[test]
    fn conversions_are_inverse_for_whole_seconds() {
        assert_eq!(secs_to_frames(1.5, 48_000), 72_000);
        assert!((frames_to_secs(72_000, 48_000) - 1.5).abs() < 1e-12);
    }
}
