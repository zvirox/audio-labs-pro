//! Public handles, summaries, and data contracts for `alp-api`.

use serde::{Deserialize, Serialize};

/// Permission tiers governing AI and automation access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionTier {
    /// Read-only access: inspect projects, read transcripts, run analysis.
    Observe = 1,
    /// Edit access: modify projects, apply effects, insert TTS, and edit tracks.
    Edit = 2,
    /// Export access: render master files, export stems, and write to exports folder.
    Export = 3,
}

impl PermissionTier {
    /// Check whether this tier can execute an operation requiring `required`.
    #[must_use]
    pub fn allows(self, required: Self) -> bool {
        self >= required
    }
}

/// Human approval mode for destructive actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalMode {
    /// Automatically apply edits (all actions remain undoable).
    Auto,
    /// Require interactive confirmation before destructive edits or file overwrites.
    ConfirmDestructive,
    /// Staged review mode: AI actions are collected for human acceptance.
    Review,
}

/// Summary description of an active project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    /// Project stable handle.
    pub id: String,
    /// User-facing project name.
    pub name: String,
    /// Current mutation revision counter.
    pub revision: u64,
    /// Audio sample rate in Hz (e.g., 48000).
    pub sample_rate: u32,
    /// Total duration in seconds.
    pub duration_secs: f64,
    /// Number of tracks.
    pub track_count: usize,
    /// Number of audio clips.
    pub clip_count: usize,
    /// Number of undo steps available.
    pub undo_steps: usize,
    /// Number of redo steps available.
    pub redo_steps: usize,
    /// Whether project has unsaved modifications.
    pub is_dirty: bool,
}

/// Summary description of a timeline track.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackSummary {
    /// Track handle (e.g., `trk_0`).
    pub id: String,
    /// Track name.
    pub name: String,
    /// Track index in timeline order.
    pub index: usize,
    /// Linear volume gain (1.0 = 0 dBFS).
    pub gain: f32,
    /// Stereo panning from -1.0 (left) to +1.0 (right).
    pub pan: f32,
    /// Whether track is muted.
    pub muted: bool,
    /// Whether track is soloed.
    pub solo: bool,
    /// Whether track is armed for recording.
    pub armed: bool,
    /// Number of clips on this track.
    pub clip_count: usize,
}

/// Summary description of an individual audio clip.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipSummary {
    /// Clip handle (e.g., `clp_1a2b`).
    pub id: String,
    /// Owning track handle.
    pub track_id: String,
    /// Source media identifier.
    pub source_id: String,
    /// Timeline start position in seconds.
    pub start_secs: f64,
    /// Clip duration in seconds.
    pub duration_secs: f64,
    /// Linear clip gain multiplier.
    pub gain: f32,
    /// Fade-in length in seconds.
    pub fade_in_secs: f64,
    /// Fade-out length in seconds.
    pub fade_out_secs: f64,
}

/// Acoustic and quality analysis summary for an audio buffer or track.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisReport {
    /// Duration of analyzed audio in seconds.
    pub duration_secs: f64,
    /// Measured integrated loudness in LUFS (ITU-R BS.1770).
    pub integrated_lufs: f32,
    /// Maximum true peak level in dBFS.
    pub true_peak_dbfs: f32,
    /// Estimated noise floor level in dBFS.
    pub noise_floor_dbfs: f32,
    /// Percentage of audio classified as silence.
    pub silence_ratio: f32,
    /// Detected digital clipping spikes count.
    pub clipping_spikes: usize,
    /// Detected electrical mains hum frequency (e.g., 50.0 or 60.0 Hz), if any.
    pub detected_hum_hz: Option<f32>,
}

/// Helper converting second offsets into a formatted timestamp string (`hh:mm:ss.mmm`).
#[must_use]
pub fn format_timestamp(secs: f64) -> String {
    let total_ms = (secs * 1000.0).round().max(0.0) as u64;
    let ms = total_ms % 1000;
    let total_s = total_ms / 1000;
    let s = total_s % 60;
    let total_m = total_s / 60;
    let m = total_m % 60;
    let h = total_m / 60;
    format!("{h:02}:{m:02}:{s:02}.{ms:03}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_tier_ordering() {
        assert!(PermissionTier::Export > PermissionTier::Edit);
        assert!(PermissionTier::Edit > PermissionTier::Observe);
        assert!(PermissionTier::Edit.allows(PermissionTier::Observe));
        assert!(!PermissionTier::Observe.allows(PermissionTier::Edit));
    }

    #[test]
    fn timestamp_formatting() {
        assert_eq!(format_timestamp(0.0), "00:00:00.000");
        assert_eq!(format_timestamp(65.123), "00:01:05.123");
        assert_eq!(format_timestamp(3661.050), "01:01:01.050");
    }
}
