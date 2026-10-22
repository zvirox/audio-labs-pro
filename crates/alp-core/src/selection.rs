//! Timeline selection state.
//!
//! A selection is not part of the saved project: it is a transient view of where
//! the user is working. The model keeps the anchor and the cursor separate so a
//! shift-click can extend in either direction, and snaps both to the current
//! grid before exposing a [`FrameRange`].

use crate::ids::TrackId;
use crate::time::FrameRange;

/// One track selection with a cursor and an optional frame grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    track_id: TrackId,
    anchor: u64,
    cursor: u64,
    snap_frames: u64,
}

impl Selection {
    /// An empty selection at `frame` on `track_id`.
    #[must_use]
    pub fn at(track_id: TrackId, frame: u64) -> Self {
        Self {
            track_id,
            anchor: frame,
            cursor: frame,
            snap_frames: 1,
        }
    }

    /// A selection between two points, snapped to `snap_frames`.
    ///
    /// `snap_frames` is clamped to at least one frame. Both ends are snapped to
    /// the nearest grid line; the range is always ordered even when the cursor
    /// is left of the anchor.
    #[must_use]
    pub fn between(track_id: TrackId, anchor: u64, cursor: u64, snap_frames: u64) -> Self {
        let snap_frames = snap_frames.max(1);
        Self {
            track_id,
            anchor: snap_to(anchor, snap_frames),
            cursor: snap_to(cursor, snap_frames),
            snap_frames,
        }
    }

    /// Track the selection belongs to.
    #[must_use]
    pub fn track_id(&self) -> TrackId {
        self.track_id
    }

    /// Fixed end of the selection.
    #[must_use]
    pub fn anchor(&self) -> u64 {
        self.anchor
    }

    /// Moving end of the selection.
    #[must_use]
    pub fn cursor(&self) -> u64 {
        self.cursor
    }

    /// Current snap grid in frames (`1` means frame-accurate).
    #[must_use]
    pub fn snap_frames(&self) -> u64 {
        self.snap_frames
    }

    /// Change the snap grid and re-snap the existing ends.
    pub fn set_snap_frames(&mut self, snap_frames: u64) {
        self.snap_frames = snap_frames.max(1);
        self.anchor = self.snap(self.anchor);
        self.cursor = self.snap(self.cursor);
    }

    /// Snap one frame to the current grid.
    #[must_use]
    pub fn snap(&self, frame: u64) -> u64 {
        snap_to(frame, self.snap_frames)
    }

    /// Move the cursor, leaving the anchor in place.
    pub fn select_to(&mut self, cursor: u64) {
        self.cursor = self.snap(cursor);
    }

    /// Collapse the selection to the current cursor.
    pub fn collapse(&mut self) {
        self.anchor = self.cursor;
    }

    /// Select the whole timeline up to `end`, snapped.
    pub fn select_all(&mut self, end: u64) {
        self.anchor = 0;
        self.cursor = self.snap(end);
    }

    /// Ordered half-open range, empty when both ends coincide.
    #[must_use]
    pub fn range(&self) -> FrameRange {
        let start = self.anchor.min(self.cursor);
        let end = self.anchor.max(self.cursor);
        FrameRange { start, end }
    }

    /// Number of frames covered.
    #[must_use]
    pub fn len(&self) -> u64 {
        self.range().len()
    }

    /// Whether the anchor and cursor coincide.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.range().is_empty()
    }

    /// Clamp the selection to a timeline of `frames` and re-snap it.
    pub fn clamp_to(&mut self, frames: u64) {
        self.anchor = self.snap(self.anchor.min(frames));
        self.cursor = self.snap(self.cursor.min(frames));
    }
}

fn snap_to(frame: u64, snap_frames: u64) -> u64 {
    if snap_frames <= 1 {
        return frame;
    }
    let half = snap_frames / 2;
    frame
        .saturating_add(half)
        .checked_div(snap_frames)
        .unwrap_or(0)
        .saturating_mul(snap_frames)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TrackId;

    #[test]
    fn range_is_ordered_and_empty_at_a_point() {
        let track = TrackId::from_u128(1);
        let selection = Selection::between(track, 900, 300, 100);
        assert_eq!(
            selection.range(),
            FrameRange {
                start: 300,
                end: 900
            }
        );
        assert_eq!(selection.len(), 600);
        assert!(!selection.is_empty());

        let point = Selection::at(track, 42);
        assert!(point.is_empty());
        assert_eq!(point.range(), FrameRange { start: 42, end: 42 });
    }

    #[test]
    fn snapping_rounds_to_the_nearest_grid_line() {
        let mut selection = Selection::between(TrackId::from_u128(1), 0, 1, 480);
        assert_eq!(selection.range(), FrameRange { start: 0, end: 0 });
        selection.select_to(239);
        assert_eq!(selection.cursor(), 0);
        selection.select_to(240);
        assert_eq!(selection.cursor(), 480);
        selection.select_to(1_000);
        assert_eq!(selection.cursor(), 960);
    }

    #[test]
    fn select_all_and_clamp() {
        let mut selection = Selection::at(TrackId::from_u128(1), 10);
        selection.set_snap_frames(100);
        selection.select_all(1_049);
        assert_eq!(
            selection.range(),
            FrameRange {
                start: 0,
                end: 1_000
            }
        );
        selection.clamp_to(800);
        assert_eq!(selection.range(), FrameRange { start: 0, end: 800 });
    }
}
