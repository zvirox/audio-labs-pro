//! Multitrack timeline DTOs and data representations.
//!
//! Exposes typed serialized structures consumed by the Tauri IPC bridge and React
//! frontend for rendering and interacting with the canvas timeline, multi-view modes,
//! spectral selection, and pan/volume automation envelopes.

use alp_dsp::{FrequencyScale, WindowType};
use serde::{Deserialize, Serialize};

/// Display mode for a multitrack timeline track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TrackViewMode {
    /// Standard amplitude waveform overview.
    #[default]
    Waveform,
    /// High-resolution STFT spectrogram heatmap.
    Spectrogram,
    /// Dual multi-view: waveform top half (50%) and spectrogram bottom half (50%).
    Split,
}

/// 2D time-frequency bounding box spectral selection on a track.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpectralSelectionDto {
    /// Start time in seconds.
    pub t_start_secs: f64,
    /// End time in seconds.
    pub t_end_secs: f64,
    /// Lower frequency boundary in Hz.
    pub f_low_hz: f32,
    /// Upper frequency boundary in Hz.
    pub f_high_hz: f32,
    /// Center frequency in Hz: f_c = sqrt(f_low * f_high) or (f_low + f_high)/2.
    pub center_freq_hz: f32,
    /// Bandwidth Q factor: Q = center_freq / (f_high - f_low).
    pub bandwidth_q: f32,
}

impl SpectralSelectionDto {
    /// Creates a calibrated spectral selection with calculated center frequency and Q factor.
    #[must_use]
    pub fn new(t_start_secs: f64, t_end_secs: f64, f_low_hz: f32, f_high_hz: f32) -> Self {
        let (t1, t2) = if t_start_secs <= t_end_secs {
            (t_start_secs, t_end_secs)
        } else {
            (t_end_secs, t_start_secs)
        };
        let (f1, f2) = if f_low_hz <= f_high_hz {
            (f_low_hz.max(1.0), f_high_hz.max(2.0))
        } else {
            (f_high_hz.max(1.0), f_low_hz.max(2.0))
        };

        let bw = (f2 - f1).max(1.0);
        let center_freq_hz = (f1 * f2).sqrt();
        let bandwidth_q = center_freq_hz / bw;

        Self {
            t_start_secs: t1,
            t_end_secs: t2,
            f_low_hz: f1,
            f_high_hz: f2,
            center_freq_hz,
            bandwidth_q,
        }
    }
}

fn default_master_gain() -> f32 {
    1.0
}

/// Full multitrack timeline presentation state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineDto {
    /// Project audio sample rate in Hz (e.g. 48000).
    pub sample_rate: u32,
    /// Master bus linear gain multiplier.
    #[serde(default = "default_master_gain")]
    pub master_gain: f32,
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
    /// Track visualization mode (Waveform, Spectrogram, or Split).
    pub view_mode: TrackViewMode,
    /// Active 2D time-frequency spectral selection box if present.
    pub spectral_selection: Option<SpectralSelectionDto>,
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
    /// Slip offset in seconds (shifts the source audio window within the fixed clip bounds).
    pub slip_offset_secs: f64,
    /// Volume automation envelope control points.
    pub envelope: Vec<TimelineEnvelopePointDto>,
    /// Pan automation envelope control points.
    pub pan_envelope: Vec<TimelinePanPointDto>,
}

/// Single control point on a clip's volume envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineEnvelopePointDto {
    /// Offset in seconds from the clip start.
    pub offset_secs: f64,
    /// Linear gain value at this point.
    pub gain: f32,
}

/// Single control point on a clip's pan automation envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelinePanPointDto {
    /// Offset in seconds from clip start.
    pub offset_secs: f64,
    /// Stereo panning value (-1.0 to 1.0).
    pub pan: f32,
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

/// Parameters for requesting spectrogram calculation from session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpectrogramRequestDto {
    /// Target clip identifier.
    pub clip_id: String,
    /// Optional FFT size override (default: 2048).
    pub fft_size: Option<usize>,
    /// Optional window function (default: hanning).
    pub window_type: Option<WindowType>,
    /// Optional frequency scaling mode (default: logarithmic).
    pub freq_scale: Option<FrequencyScale>,
    /// Optional target frequency bins (default: 256).
    pub num_freq_bins: Option<usize>,
}
