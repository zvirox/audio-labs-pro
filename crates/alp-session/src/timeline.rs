//! Multitrack timeline DTOs and data representations.
//!
//! Exposes typed serialized structures consumed by the Tauri IPC bridge and React
//! frontend for rendering and interacting with the canvas timeline.

use serde::{Deserialize, Serialize};

/// Full multitrack timeline presentation state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineDto {
    /// Project audio sample rate in Hz (e.g. 48000).
    pub sample_rate: u32,
    /// Total duration of the timeline in seconds.
    pub duration_secs: f64,
    /// Ordered list of tracks and contained clips.
    pub tracks: Vec<TimelineTrackDto>,
    /// Marker points along the timeline.
    pub markers: Vec<TimelineMarkerDto>,
    /// Active loop boundaries in seconds if enabled.
    pub loop_range: Option<TimelineLoopRangeDto>,
    /// Active edit selection range if non-empty.
    pub selection: Option<TimelineSelectionDto>,
    /// Currently focused or selected clip identifier.
    pub selected_clip_id: Option<String>,
    /// Whether an undo action is currently available.
    pub can_undo: bool,
    /// Whether a redo action is currently available.
    pub can_redo: bool,
    /// Number of undo steps on the history stack.
    pub undo_steps: usize,
    /// Number of redo steps on the history stack.
    pub redo_steps: usize,
    /// Monotonically increasing revision counter.
    pub revision: u64,
}

/// A single audio or label track on the timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineTrackDto {
    /// Track unique identifier string.
    pub id: String,
    /// Human-readable track title.
    pub name: String,
    /// Linear track gain multiplier.
    pub gain: f32,
    /// Stereo panning position (-1.0 to 1.0).
    pub pan: f32,
    /// Mute status.
    pub muted: bool,
    /// Solo status.
    pub solo: bool,
    /// Record armed status.
    pub armed: bool,
    /// Non-destructive audio clips placed on this track.
    pub clips: Vec<TimelineClipDto>,
}

/// A non-destructive audio clip reference on the timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineClipDto {
    /// Stable clip identifier.
    pub id: String,
    /// Underlying audio source identifier.
    pub source_id: String,
    /// Source or clip display name.
    pub name: String,
    /// Start position on the timeline in seconds.
    pub timeline_start_secs: f64,
    /// Duration of the clip in seconds.
    pub duration_secs: f64,
    /// Start offset into the source audio buffer in seconds.
    pub source_start_secs: f64,
    /// Linear clip gain multiplier.
    pub gain: f32,
    /// Fade-in length in seconds.
    pub fade_in_secs: f64,
    /// Fade-out length in seconds.
    pub fade_out_secs: f64,
    /// Volume automation envelope control points.
    pub envelope: Vec<TimelineEnvelopePointDto>,
}

/// Single control point on a clip's volume envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEnvelopePointDto {
    /// Offset in seconds from the clip start.
    pub offset_secs: f64,
    /// Linear gain value at this point.
    pub gain: f32,
}

/// Timeline cue marker or bookmark.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineMarkerDto {
    /// Stable marker identifier.
    pub id: String,
    /// Timestamp in seconds.
    pub time_secs: f64,
    /// Descriptive text label.
    pub label: String,
}

/// Loop playback range boundaries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineLoopRangeDto {
    /// Loop start time in seconds.
    pub start_secs: f64,
    /// Loop end time in seconds.
    pub end_secs: f64,
}

/// In-out edit selection range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineSelectionDto {
    /// Target track identifier if bounded to a track.
    pub track_id: Option<String>,
    /// Selection start time in seconds.
    pub start_secs: f64,
    /// Selection end time in seconds.
    pub end_secs: f64,
}
