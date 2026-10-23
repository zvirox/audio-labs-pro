//! Timeline annotations: named markers, named regions and the loop range.
//!
//! These annotations are project data, not rendered audio. They drive the
//! timeline ruler, loop range and later editing features, and are entirely
//! undoable through commands in [`crate::commands::markers`].

use serde::{Deserialize, Serialize};

use crate::ids::{MarkerId, RegionId};
use crate::time::FrameRange;

/// A named timeline point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Marker {
    /// Stable identifier.
    pub id: MarkerId,
    /// Timeline frame.
    pub frame: u64,
    /// Short display label.
    pub label: String,
}

/// A named timeline range.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Region {
    /// Stable identifier.
    pub id: RegionId,
    /// Timeline range in project frames.
    pub range: FrameRange,
    /// Short display label.
    pub label: String,
}

impl Marker {
    /// Create a marker.
    #[must_use]
    pub fn new(id: MarkerId, frame: u64, label: impl Into<String>) -> Self {
        Self {
            id,
            frame,
            label: label.into(),
        }
    }
}

impl Region {
    /// Create a region.
    #[must_use]
    pub fn new(id: RegionId, range: FrameRange, label: impl Into<String>) -> Self {
        Self {
            id,
            range,
            label: label.into(),
        }
    }
}
