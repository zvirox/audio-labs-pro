//! The project model: projects, sources, tracks and clips.
//!
//! The model is deliberately plain data. Every mutation performed by the
//! application goes through a [`crate::Command`] so undo/redo can invert it.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::ids::{ClipId, ProjectId, SourceId, TrackId};
use crate::time::{FrameRange, frames_to_secs, secs_to_frames};

/// Default project sample rate (ADR 0004).
pub const DEFAULT_SAMPLE_RATE: u32 = 48_000;
/// Lowest accepted project sample rate.
pub const MIN_SAMPLE_RATE: u32 = 8_000;
/// Highest accepted project sample rate.
pub const MAX_SAMPLE_RATE: u32 = 384_000;

/// Milliseconds since the Unix epoch, saturating at zero on clock errors.
#[must_use]
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as u64)
}

/// A complete non-destructive editing project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    /// Stable project identifier.
    pub id: ProjectId,
    /// Human-readable project name.
    pub name: String,
    /// Sample rate shared by every timeline position in the project.
    pub sample_rate: u32,
    /// Creation time in Unix milliseconds.
    pub created_at_ms: u64,
    /// Last modification time in Unix milliseconds.
    pub updated_at_ms: u64,
    /// Imported source media.
    pub sources: Vec<Source>,
    /// Tracks in display order.
    pub tracks: Vec<Track>,
}

impl Project {
    /// Create an empty project at the default sample rate.
    pub fn new(name: impl Into<String>) -> Result<Self, CoreError> {
        Self::new_with_rate(name, DEFAULT_SAMPLE_RATE)
    }

    /// Create an empty project at an explicit sample rate.
    pub fn new_with_rate(name: impl Into<String>, sample_rate: u32) -> Result<Self, CoreError> {
        Self::new_with_id(ProjectId::new(), name, sample_rate)
    }

    /// Create an empty project with a deterministic identifier (tests, fixtures).
    pub fn new_with_id(
        id: ProjectId,
        name: impl Into<String>,
        sample_rate: u32,
    ) -> Result<Self, CoreError> {
        if !(MIN_SAMPLE_RATE..=MAX_SAMPLE_RATE).contains(&sample_rate) {
            return Err(CoreError::InvalidSampleRate(sample_rate));
        }
        let now = now_ms();
        Ok(Self {
            id,
            name: name.into(),
            sample_rate,
            created_at_ms: now,
            updated_at_ms: now,
            sources: Vec::new(),
            tracks: Vec::new(),
        })
    }

    /// Mark the project as modified now.
    pub fn touch(&mut self) {
        self.updated_at_ms = now_ms();
    }

    /// Find a source by id.
    #[must_use]
    pub fn source(&self, id: SourceId) -> Option<&Source> {
        self.sources.iter().find(|source| source.id == id)
    }

    /// Find a source by id, mutably.
    pub fn source_mut(&mut self, id: SourceId) -> Option<&mut Source> {
        self.sources.iter_mut().find(|source| source.id == id)
    }

    /// Find a track by id.
    #[must_use]
    pub fn track(&self, id: TrackId) -> Option<&Track> {
        self.tracks.iter().find(|track| track.id == id)
    }

    /// Find a track by id, mutably.
    pub fn track_mut(&mut self, id: TrackId) -> Option<&mut Track> {
        self.tracks.iter_mut().find(|track| track.id == id)
    }

    /// Find a clip and the track that owns it.
    #[must_use]
    pub fn clip(&self, id: ClipId) -> Option<(&Track, &Clip)> {
        self.tracks.iter().find_map(|track| {
            track
                .clips
                .iter()
                .find(|clip| clip.id == id)
                .map(|clip| (track, clip))
        })
    }

    /// Find a clip by id, mutably.
    pub fn clip_mut(&mut self, id: ClipId) -> Option<&mut Clip> {
        self.tracks
            .iter_mut()
            .flat_map(|track| track.clips.iter_mut())
            .find(|clip| clip.id == id)
    }

    /// Track that owns a clip.
    #[must_use]
    pub fn track_of_clip(&self, id: ClipId) -> Option<&Track> {
        self.tracks
            .iter()
            .find(|track| track.clips.iter().any(|clip| clip.id == id))
    }

    /// Last frame occupied by any clip, in project frames.
    ///
    /// A clip whose source runs at a different rate is resampled to the project
    /// rate before playback, so its timeline length is scaled here as well.
    #[must_use]
    pub fn duration_frames(&self) -> u64 {
        self.tracks
            .iter()
            .flat_map(|track| track.clips.iter())
            .map(|clip| self.clip_timeline_end(clip))
            .max()
            .unwrap_or(0)
    }

    /// Clip length on the project timeline.
    ///
    /// `Clip::len` is in source frames. This method converts it to project
    /// frames so mixed-rate sources report the duration the engine will play.
    #[must_use]
    pub fn clip_timeline_len(&self, clip: &Clip) -> u64 {
        let rate = self
            .source(clip.source_id)
            .map_or(self.sample_rate, |source| source.sample_rate);
        if rate == self.sample_rate {
            clip.len()
        } else {
            secs_to_frames(frames_to_secs(clip.len(), rate), self.sample_rate)
        }
    }

    /// First frame after a clip on the project timeline.
    #[must_use]
    pub fn clip_timeline_end(&self, clip: &Clip) -> u64 {
        clip.timeline_start
            .saturating_add(self.clip_timeline_len(clip))
    }

    /// Total number of clips across all tracks.
    #[must_use]
    pub fn clip_count(&self) -> usize {
        self.tracks.iter().map(|track| track.clips.len()).sum()
    }

    /// Check structural invariants. Called after every undoable command in tests
    /// and before saving.
    pub fn validate(&self) -> Result<(), CoreError> {
        if !(MIN_SAMPLE_RATE..=MAX_SAMPLE_RATE).contains(&self.sample_rate) {
            return Err(CoreError::InvalidSampleRate(self.sample_rate));
        }
        if self.name.trim().is_empty() {
            return Err(CoreError::validation("project name is empty"));
        }

        let mut source_ids = HashSet::new();
        for source in &self.sources {
            if !source_ids.insert(source.id) {
                return Err(CoreError::validation(format!(
                    "duplicate source id {}",
                    source.id
                )));
            }
            source.validate()?;
        }

        let source_frames: HashMap<SourceId, u64> = self
            .sources
            .iter()
            .map(|source| (source.id, source.frames))
            .collect();

        let mut track_ids = HashSet::new();
        let mut clip_ids = HashSet::new();
        for track in &self.tracks {
            if !track_ids.insert(track.id) {
                return Err(CoreError::validation(format!(
                    "duplicate track id {}",
                    track.id
                )));
            }
            track.validate(&source_ids, &source_frames, &mut clip_ids)?;
        }
        Ok(())
    }
}

/// Imported, immutable source media.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Source {
    /// Stable identifier.
    pub id: SourceId,
    /// Display name.
    pub name: String,
    /// Path relative to the project root (`media/...`).
    pub relative_path: String,
    /// Native sample rate of the file.
    pub sample_rate: u32,
    /// Native channel count.
    pub channels: u16,
    /// Native length in frames.
    pub frames: u64,
    /// Largest absolute sample value measured at import.
    #[serde(default)]
    pub peak: f32,
    /// Import time in Unix milliseconds.
    pub imported_at_ms: u64,
}

impl Source {
    /// Build a source record.
    pub fn new(
        id: SourceId,
        name: impl Into<String>,
        relative_path: impl Into<String>,
        sample_rate: u32,
        channels: u16,
        frames: u64,
    ) -> Result<Self, CoreError> {
        if !(MIN_SAMPLE_RATE..=MAX_SAMPLE_RATE).contains(&sample_rate) {
            return Err(CoreError::InvalidSampleRate(sample_rate));
        }
        Ok(Self {
            id,
            name: name.into(),
            relative_path: relative_path.into(),
            sample_rate,
            channels,
            frames,
            peak: 0.0,
            imported_at_ms: now_ms(),
        })
    }

    /// Record the measured source peak for normalization.
    #[must_use]
    pub fn with_peak(mut self, peak: f32) -> Self {
        self.peak = peak;
        self
    }

    fn validate(&self) -> Result<(), CoreError> {
        if self.relative_path.trim().is_empty() {
            return Err(CoreError::validation("source path is empty"));
        }
        if self.channels == 0 {
            return Err(CoreError::validation("source has zero channels"));
        }
        if self.frames == 0 {
            return Err(CoreError::validation("source has zero frames"));
        }
        if !(MIN_SAMPLE_RATE..=MAX_SAMPLE_RATE).contains(&self.sample_rate) {
            return Err(CoreError::InvalidSampleRate(self.sample_rate));
        }
        if !self.peak.is_finite() || self.peak < 0.0 {
            return Err(CoreError::validation("source peak is invalid"));
        }
        Ok(())
    }
}

/// One timeline track.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    /// Stable identifier.
    pub id: TrackId,
    /// Display name.
    pub name: String,
    /// Linear track gain (`1.0` is unity).
    pub gain: f32,
    /// Stereo pan, `-1.0` left to `1.0` right.
    pub pan: f32,
    /// Whether the track is muted.
    pub muted: bool,
    /// Whether the track is soloed.
    pub solo: bool,
    /// Display colour as sRGB bytes.
    pub color: [u8; 3],
    /// Clips in timeline order.
    pub clips: Vec<Clip>,
}

impl Track {
    /// Create an empty track with default settings.
    #[must_use]
    pub fn new(id: TrackId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            gain: 1.0,
            pan: 0.0,
            muted: false,
            solo: false,
            color: [86, 132, 196],
            clips: Vec::new(),
        }
    }

    fn validate(
        &self,
        source_ids: &HashSet<SourceId>,
        source_frames: &HashMap<SourceId, u64>,
        clip_ids: &mut HashSet<ClipId>,
    ) -> Result<(), CoreError> {
        if self.name.trim().is_empty() {
            return Err(CoreError::validation("track name is empty"));
        }
        if !self.gain.is_finite() || self.gain < 0.0 {
            return Err(CoreError::validation(
                "track gain is not a non-negative number",
            ));
        }
        if !self.pan.is_finite() || !(-1.0..=1.0).contains(&self.pan) {
            return Err(CoreError::validation("track pan is outside -1..=1"));
        }
        for clip in &self.clips {
            if !clip_ids.insert(clip.id) {
                return Err(CoreError::validation(format!(
                    "duplicate clip id {}",
                    clip.id
                )));
            }
            clip.validate(source_ids, source_frames)?;
        }
        Ok(())
    }
}

/// A non-destructive reference to a region of a [`Source`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Clip {
    /// Stable identifier.
    pub id: ClipId,
    /// Source this clip reads from.
    pub source_id: SourceId,
    /// Region of the source, in source frames.
    pub source_range: FrameRange,
    /// Timeline position of the first sample, in project frames.
    pub timeline_start: u64,
    /// Linear clip gain (`1.0` is unity).
    pub gain: f32,
    /// Fade-in length in frames.
    pub fade_in_frames: u64,
    /// Fade-out length in frames.
    pub fade_out_frames: u64,
    /// Volume automation relative to the clip start.
    pub envelope: Vec<EnvelopePoint>,
    /// Play the source region backwards.
    #[serde(default)]
    pub reversed: bool,
    /// Flip the sample sign during playback and render.
    #[serde(default)]
    pub inverted: bool,
    /// Effect chain placeholders (Phase 5 fills these in).
    pub effects: Vec<EffectRef>,
}

impl Clip {
    /// Create a clip with unity gain and no fades.
    #[must_use]
    pub fn new(
        id: ClipId,
        source_id: SourceId,
        source_range: FrameRange,
        timeline_start: u64,
    ) -> Self {
        Self {
            id,
            source_id,
            source_range,
            timeline_start,
            gain: 1.0,
            fade_in_frames: 0,
            fade_out_frames: 0,
            envelope: Vec::new(),
            reversed: false,
            inverted: false,
            effects: Vec::new(),
        }
    }

    /// Clip length in source frames.
    #[must_use]
    pub fn len(&self) -> u64 {
        self.source_range.len()
    }

    /// Whether the clip contains no frames.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.source_range.is_empty()
    }

    /// First frame after the clip assuming the source and project rates match.
    ///
    /// For a mixed-rate source use [`Project::clip_timeline_end`], which scales
    /// the source length to project frames.
    #[must_use]
    pub fn timeline_end(&self) -> u64 {
        self.timeline_start.saturating_add(self.len())
    }

    /// Timeline range assuming the source and project rates match.
    ///
    /// For a mixed-rate source use [`Project::clip_timeline_end`].
    #[must_use]
    pub fn timeline_range(&self) -> FrameRange {
        FrameRange {
            start: self.timeline_start,
            end: self.timeline_end(),
        }
    }

    fn validate(
        &self,
        source_ids: &HashSet<SourceId>,
        source_frames: &HashMap<SourceId, u64>,
    ) -> Result<(), CoreError> {
        if !source_ids.contains(&self.source_id) {
            return Err(CoreError::validation(format!(
                "clip {} references missing source {}",
                self.id, self.source_id
            )));
        }
        if self.source_range.end < self.source_range.start {
            return Err(CoreError::validation(format!(
                "clip {} has an inverted source range",
                self.id
            )));
        }
        if source_frames
            .get(&self.source_id)
            .is_some_and(|frames| self.source_range.end > *frames)
        {
            return Err(CoreError::validation(format!(
                "clip {} extends past the end of its source",
                self.id
            )));
        }
        if self.is_empty() {
            return Err(CoreError::validation(format!(
                "clip {} has an empty source range",
                self.id
            )));
        }
        if !self.gain.is_finite() || self.gain < 0.0 {
            return Err(CoreError::validation(
                "clip gain is not a non-negative number",
            ));
        }
        if self.fade_in_frames.saturating_add(self.fade_out_frames) > self.len() {
            return Err(CoreError::validation(format!(
                "clip {} fades are longer than the clip",
                self.id
            )));
        }
        let mut previous = 0;
        for (index, point) in self.envelope.iter().enumerate() {
            if !point.gain.is_finite() || point.gain < 0.0 {
                return Err(CoreError::validation("envelope gain is invalid"));
            }
            if point.frame > self.len() {
                return Err(CoreError::validation("envelope point is past the clip end"));
            }
            if index > 0 && point.frame < previous {
                return Err(CoreError::validation("envelope points are not sorted"));
            }
            previous = point.frame;
        }
        Ok(())
    }
}

/// One point on a clip's volume automation curve.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EnvelopePoint {
    /// Frame relative to the clip start.
    pub frame: u64,
    /// Linear gain multiplier.
    pub gain: f32,
}

/// Placeholder for an effect inserted on a clip.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectRef {
    /// Stable identifier used by the effects UI.
    pub id: String,
    /// Effect type understood by `alp-dsp`.
    pub kind: String,
    /// Whether the effect is bypassed.
    pub enabled: bool,
    /// Serialized parameters.
    pub params: BTreeMap<String, f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_source() -> Source {
        Source::new(
            SourceId::from_u128(1),
            "voice.wav",
            "media/voice.wav",
            48_000,
            1,
            48_000,
        )
        .expect("source")
    }

    #[test]
    fn empty_project_is_valid() {
        let project = Project::new("untitled").expect("project");
        project.validate().expect("valid");
        assert_eq!(project.duration_frames(), 0);
    }

    #[test]
    fn invalid_sample_rate_is_rejected() {
        assert!(Project::new_with_rate("x", 1).is_err());
        assert!(Project::new_with_rate("x", 999_999).is_err());
    }

    #[test]
    fn clip_past_source_end_is_invalid() {
        let mut project = Project::new("x").expect("project");
        project.sources.push(sample_source());
        let mut track = Track::new(TrackId::from_u128(2), "Track 1");
        track.clips.push(Clip::new(
            ClipId::from_u128(3),
            SourceId::from_u128(1),
            FrameRange::new(0, 48_001).expect("range"),
            0,
        ));
        project.tracks.push(track);
        assert!(project.validate().is_err());
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let mut project = Project::new("x").expect("project");
        let track = Track::new(TrackId::from_u128(2), "Track 1");
        project.tracks.push(track.clone());
        project.tracks.push(track);
        assert!(project.validate().is_err());
    }
}
