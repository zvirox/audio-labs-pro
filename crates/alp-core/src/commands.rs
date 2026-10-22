//! Concrete undoable commands for the project model.
//!
//! Constructors take ownership of the new value and `apply` stores whatever it
//! needs to restore the previous state. A command may only be applied once per
//! undo/redo cycle, which is what [`crate::CommandStack`] guarantees.

use crate::command::{Command, CommandError};
use crate::ids::{ClipId, SourceId, TrackId};
use crate::project::{Clip, EffectRef, EnvelopePoint, Project, Source, Track};
use crate::time::FrameRange;

fn track_index(project: &Project, id: TrackId) -> Result<usize, CommandError> {
    project
        .tracks
        .iter()
        .position(|track| track.id == id)
        .ok_or(CommandError::TrackNotFound(id))
}

fn clip_location(project: &Project, id: ClipId) -> Result<(usize, usize), CommandError> {
    for (track_index, track) in project.tracks.iter().enumerate() {
        if let Some(clip_index) = track.clips.iter().position(|clip| clip.id == id) {
            return Ok((track_index, clip_index));
        }
    }
    Err(CommandError::ClipNotFound(id))
}

fn check_index(index: usize, len: usize) -> Result<(), CommandError> {
    if index > len {
        return Err(CommandError::IndexOutOfBounds { index, len });
    }
    Ok(())
}

fn check_name(name: &str) -> Result<(), CommandError> {
    if name.trim().is_empty() {
        return Err(CommandError::Invalid("name is empty".into()));
    }
    Ok(())
}

/// Insert an existing track at `index`.
#[derive(Debug)]
pub struct AddTrack {
    track: Track,
    index: usize,
    inserted: Option<usize>,
}

impl AddTrack {
    /// Build the command.
    #[must_use]
    pub fn new(track: Track, index: usize) -> Self {
        Self {
            track,
            index,
            inserted: None,
        }
    }
}

impl Command for AddTrack {
    fn label(&self) -> &'static str {
        "Add track"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        check_index(self.index, project.tracks.len())?;
        if project.tracks.iter().any(|track| track.id == self.track.id) {
            return Err(CommandError::Duplicate(format!("track {}", self.track.id)));
        }
        project.tracks.insert(self.index, self.track.clone());
        self.inserted = Some(self.index);
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let index = self
            .inserted
            .take()
            .ok_or(CommandError::InconsistentState("track was not added"))?;
        let removed = project
            .tracks
            .get(index)
            .ok_or(CommandError::TrackNotFound(self.track.id))?;
        if removed.id != self.track.id {
            return Err(CommandError::InconsistentState("track index changed"));
        }
        project.tracks.remove(index);
        Ok(())
    }
}

/// Remove a track and remember it for undo.
#[derive(Debug, Default)]
pub struct RemoveTrack {
    track_id: TrackId,
    removed: Option<(usize, Track)>,
}

impl RemoveTrack {
    /// Build the command.
    #[must_use]
    pub fn new(track_id: TrackId) -> Self {
        Self {
            track_id,
            removed: None,
        }
    }
}

impl Command for RemoveTrack {
    fn label(&self) -> &'static str {
        "Remove track"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let index = track_index(project, self.track_id)?;
        let track = project.tracks.remove(index);
        self.removed = Some((index, track));
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let (index, track) = self
            .removed
            .take()
            .ok_or(CommandError::InconsistentState("track was not removed"))?;
        check_index(index, project.tracks.len())?;
        project.tracks.insert(index, track);
        Ok(())
    }
}

/// Rename a track.
#[derive(Debug)]
pub struct RenameTrack {
    track_id: TrackId,
    name: String,
    previous: Option<String>,
}

impl RenameTrack {
    /// Build the command.
    #[must_use]
    pub fn new(track_id: TrackId, name: impl Into<String>) -> Self {
        Self {
            track_id,
            name: name.into(),
            previous: None,
        }
    }
}

impl Command for RenameTrack {
    fn label(&self) -> &'static str {
        "Rename track"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        check_name(&self.name)?;
        let index = track_index(project, self.track_id)?;
        let track = &mut project.tracks[index];
        self.previous = Some(std::mem::replace(&mut track.name, self.name.clone()));
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let previous = self
            .previous
            .take()
            .ok_or(CommandError::InconsistentState("track was not renamed"))?;
        let index = track_index(project, self.track_id)?;
        project.tracks[index].name = previous;
        Ok(())
    }
}

/// Set a track's linear gain.
#[derive(Debug)]
pub struct SetTrackGain {
    track_id: TrackId,
    gain: f32,
    previous: Option<f32>,
}

impl SetTrackGain {
    /// Build the command.
    #[must_use]
    pub fn new(track_id: TrackId, gain: f32) -> Self {
        Self {
            track_id,
            gain,
            previous: None,
        }
    }
}

impl Command for SetTrackGain {
    fn label(&self) -> &'static str {
        "Set track gain"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        if !self.gain.is_finite() || self.gain < 0.0 {
            return Err(CommandError::Invalid(
                "gain must be finite and non-negative".into(),
            ));
        }
        let index = track_index(project, self.track_id)?;
        let track = &mut project.tracks[index];
        self.previous = Some(std::mem::replace(&mut track.gain, self.gain));
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let previous = self
            .previous
            .take()
            .ok_or(CommandError::InconsistentState("gain was not set"))?;
        let index = track_index(project, self.track_id)?;
        project.tracks[index].gain = previous;
        Ok(())
    }
}

/// Set a track's pan position.
#[derive(Debug)]
pub struct SetTrackPan {
    track_id: TrackId,
    pan: f32,
    previous: Option<f32>,
}

impl SetTrackPan {
    /// Build the command.
    #[must_use]
    pub fn new(track_id: TrackId, pan: f32) -> Self {
        Self {
            track_id,
            pan,
            previous: None,
        }
    }
}

impl Command for SetTrackPan {
    fn label(&self) -> &'static str {
        "Set track pan"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        if !self.pan.is_finite() || !(-1.0..=1.0).contains(&self.pan) {
            return Err(CommandError::Invalid("pan must be within -1..=1".into()));
        }
        let index = track_index(project, self.track_id)?;
        let track = &mut project.tracks[index];
        self.previous = Some(std::mem::replace(&mut track.pan, self.pan));
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let previous = self
            .previous
            .take()
            .ok_or(CommandError::InconsistentState("pan was not set"))?;
        let index = track_index(project, self.track_id)?;
        project.tracks[index].pan = previous;
        Ok(())
    }
}

/// Set a track's mute flag.
#[derive(Debug)]
pub struct SetTrackMuted {
    track_id: TrackId,
    muted: bool,
    previous: Option<bool>,
}

impl SetTrackMuted {
    /// Build the command.
    #[must_use]
    pub fn new(track_id: TrackId, muted: bool) -> Self {
        Self {
            track_id,
            muted,
            previous: None,
        }
    }
}

impl Command for SetTrackMuted {
    fn label(&self) -> &'static str {
        "Mute track"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let index = track_index(project, self.track_id)?;
        let track = &mut project.tracks[index];
        self.previous = Some(std::mem::replace(&mut track.muted, self.muted));
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let previous = self
            .previous
            .take()
            .ok_or(CommandError::InconsistentState("mute was not set"))?;
        let index = track_index(project, self.track_id)?;
        project.tracks[index].muted = previous;
        Ok(())
    }
}

/// Set a track's solo flag.
#[derive(Debug)]
pub struct SetTrackSolo {
    track_id: TrackId,
    solo: bool,
    previous: Option<bool>,
}

impl SetTrackSolo {
    /// Build the command.
    #[must_use]
    pub fn new(track_id: TrackId, solo: bool) -> Self {
        Self {
            track_id,
            solo,
            previous: None,
        }
    }
}

impl Command for SetTrackSolo {
    fn label(&self) -> &'static str {
        "Solo track"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let index = track_index(project, self.track_id)?;
        let track = &mut project.tracks[index];
        self.previous = Some(std::mem::replace(&mut track.solo, self.solo));
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let previous = self
            .previous
            .take()
            .ok_or(CommandError::InconsistentState("solo was not set"))?;
        let index = track_index(project, self.track_id)?;
        project.tracks[index].solo = previous;
        Ok(())
    }
}

/// Move a track to another position in the display order.
#[derive(Debug)]
pub struct MoveTrack {
    track_id: TrackId,
    to_index: usize,
    previous: Option<usize>,
}

impl MoveTrack {
    /// Build the command.
    #[must_use]
    pub fn new(track_id: TrackId, to_index: usize) -> Self {
        Self {
            track_id,
            to_index,
            previous: None,
        }
    }
}

impl Command for MoveTrack {
    fn label(&self) -> &'static str {
        "Move track"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let from = track_index(project, self.track_id)?;
        if self.to_index >= project.tracks.len() {
            return Err(CommandError::IndexOutOfBounds {
                index: self.to_index,
                len: project.tracks.len(),
            });
        }
        let track = project.tracks.remove(from);
        project.tracks.insert(self.to_index, track);
        self.previous = Some(from);
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let previous = self
            .previous
            .take()
            .ok_or(CommandError::InconsistentState("track was not moved"))?;
        let from = track_index(project, self.track_id)?;
        let track = project.tracks.remove(from);
        project.tracks.insert(previous, track);
        Ok(())
    }
}

/// Insert an imported source at `index`.
#[derive(Debug)]
pub struct AddSource {
    source: Source,
    index: usize,
    inserted: bool,
}

impl AddSource {
    /// Build the command.
    #[must_use]
    pub fn new(source: Source, index: usize) -> Self {
        Self {
            source,
            index,
            inserted: false,
        }
    }
}

impl Command for AddSource {
    fn label(&self) -> &'static str {
        "Add source"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        check_index(self.index, project.sources.len())?;
        if project
            .sources
            .iter()
            .any(|source| source.id == self.source.id)
        {
            return Err(CommandError::Duplicate(format!(
                "source {}",
                self.source.id
            )));
        }
        project.sources.insert(self.index, self.source.clone());
        self.inserted = true;
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        if !self.inserted {
            return Err(CommandError::InconsistentState("source was not added"));
        }
        let removed = project
            .sources
            .get(self.index)
            .ok_or(CommandError::SourceNotFound(self.source.id))?;
        if removed.id != self.source.id {
            return Err(CommandError::InconsistentState("source index changed"));
        }
        project.sources.remove(self.index);
        self.inserted = false;
        Ok(())
    }
}

/// Remove a source that is not referenced by any clip.
#[derive(Debug, Default)]
pub struct RemoveSource {
    source_id: SourceId,
    removed: Option<(usize, Source)>,
}

impl RemoveSource {
    /// Build the command.
    #[must_use]
    pub fn new(source_id: SourceId) -> Self {
        Self {
            source_id,
            removed: None,
        }
    }
}

impl Command for RemoveSource {
    fn label(&self) -> &'static str {
        "Remove source"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        if project
            .tracks
            .iter()
            .flat_map(|track| track.clips.iter())
            .any(|clip| clip.source_id == self.source_id)
        {
            return Err(CommandError::Invalid(
                "source is still used by a clip".into(),
            ));
        }
        let index = project
            .sources
            .iter()
            .position(|source| source.id == self.source_id)
            .ok_or(CommandError::SourceNotFound(self.source_id))?;
        self.removed = Some((index, project.sources.remove(index)));
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let (index, source) = self
            .removed
            .take()
            .ok_or(CommandError::InconsistentState("source was not removed"))?;
        check_index(index, project.sources.len())?;
        project.sources.insert(index, source);
        Ok(())
    }
}

/// Insert a clip into a track.
#[derive(Debug)]
pub struct AddClip {
    track_id: TrackId,
    clip: Clip,
    index: usize,
    inserted: bool,
}

impl AddClip {
    /// Build the command.
    #[must_use]
    pub fn new(track_id: TrackId, clip: Clip, index: usize) -> Self {
        Self {
            track_id,
            clip,
            index,
            inserted: false,
        }
    }
}

impl Command for AddClip {
    fn label(&self) -> &'static str {
        "Add clip"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let track_index = track_index(project, self.track_id)?;
        check_index(self.index, project.tracks[track_index].clips.len())?;
        if project.source(self.clip.source_id).is_none() {
            return Err(CommandError::SourceNotFound(self.clip.source_id));
        }
        if project.clip(self.clip.id).is_some() {
            return Err(CommandError::Duplicate(format!("clip {}", self.clip.id)));
        }
        project.tracks[track_index]
            .clips
            .insert(self.index, self.clip.clone());
        self.inserted = true;
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        if !self.inserted {
            return Err(CommandError::InconsistentState("clip was not added"));
        }
        let track_index = track_index(project, self.track_id)?;
        let removed = project.tracks[track_index]
            .clips
            .get(self.index)
            .ok_or(CommandError::ClipNotFound(self.clip.id))?;
        if removed.id != self.clip.id {
            return Err(CommandError::InconsistentState("clip index changed"));
        }
        project.tracks[track_index].clips.remove(self.index);
        self.inserted = false;
        Ok(())
    }
}

/// Remove a clip and remember its location.
#[derive(Debug, Default)]
pub struct RemoveClip {
    clip_id: ClipId,
    removed: Option<(usize, usize, Clip)>,
}

impl RemoveClip {
    /// Build the command.
    #[must_use]
    pub fn new(clip_id: ClipId) -> Self {
        Self {
            clip_id,
            removed: None,
        }
    }
}

impl Command for RemoveClip {
    fn label(&self) -> &'static str {
        "Remove clip"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let (track_index, clip_index) = clip_location(project, self.clip_id)?;
        let clip = project.tracks[track_index].clips.remove(clip_index);
        self.removed = Some((track_index, clip_index, clip));
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let (track_index, clip_index, clip) = self
            .removed
            .take()
            .ok_or(CommandError::InconsistentState("clip was not removed"))?;
        let track = project
            .tracks
            .get_mut(track_index)
            .ok_or(CommandError::InconsistentState("track index changed"))?;
        check_index(clip_index, track.clips.len())?;
        track.clips.insert(clip_index, clip);
        Ok(())
    }
}

/// Move a clip to another track, index and timeline position.
#[derive(Debug)]
pub struct MoveClip {
    clip_id: ClipId,
    target_track: TrackId,
    target_index: usize,
    timeline_start: u64,
    previous: Option<(TrackId, usize, u64)>,
}

impl MoveClip {
    /// Build the command.
    #[must_use]
    pub fn new(
        clip_id: ClipId,
        target_track: TrackId,
        target_index: usize,
        timeline_start: u64,
    ) -> Self {
        Self {
            clip_id,
            target_track,
            target_index,
            timeline_start,
            previous: None,
        }
    }
}

impl Command for MoveClip {
    fn label(&self) -> &'static str {
        "Move clip"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let (source_track_index, source_clip_index) = clip_location(project, self.clip_id)?;
        let target_track_index = track_index(project, self.target_track)?;
        let source_track_id = project.tracks[source_track_index].id;
        let previous_start =
            project.tracks[source_track_index].clips[source_clip_index].timeline_start;

        let target_len_after_removal = if source_track_index == target_track_index {
            project.tracks[target_track_index].clips.len() - 1
        } else {
            project.tracks[target_track_index].clips.len()
        };
        check_index(self.target_index, target_len_after_removal)?;

        let mut clip = project.tracks[source_track_index]
            .clips
            .remove(source_clip_index);
        clip.timeline_start = self.timeline_start;
        project.tracks[target_track_index]
            .clips
            .insert(self.target_index, clip);
        self.previous = Some((source_track_id, source_clip_index, previous_start));
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let (source_track_id, source_index, previous_start) = self
            .previous
            .take()
            .ok_or(CommandError::InconsistentState("clip was not moved"))?;
        let (current_track_index, current_clip_index) = clip_location(project, self.clip_id)?;
        let mut clip = project.tracks[current_track_index]
            .clips
            .remove(current_clip_index);
        clip.timeline_start = previous_start;
        let source_track_index = track_index(project, source_track_id)?;
        check_index(source_index, project.tracks[source_track_index].clips.len())?;
        project.tracks[source_track_index]
            .clips
            .insert(source_index, clip);
        Ok(())
    }
}

/// Change a clip's source range and timeline position.
#[derive(Debug)]
pub struct TrimClip {
    clip_id: ClipId,
    source_range: FrameRange,
    timeline_start: u64,
    previous: Option<(FrameRange, u64, u64, u64)>,
}

impl TrimClip {
    /// Build the command.
    #[must_use]
    pub fn new(clip_id: ClipId, source_range: FrameRange, timeline_start: u64) -> Self {
        Self {
            clip_id,
            source_range,
            timeline_start,
            previous: None,
        }
    }
}

impl Command for TrimClip {
    fn label(&self) -> &'static str {
        "Trim clip"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        if self.source_range.is_empty() || self.source_range.end < self.source_range.start {
            return Err(CommandError::Invalid(
                "trim range is empty or inverted".into(),
            ));
        }
        let (source_id, _) = project
            .clip(self.clip_id)
            .map(|(_, clip)| (clip.source_id, clip.len()))
            .ok_or(CommandError::ClipNotFound(self.clip_id))?;
        let source_frames = project
            .source(source_id)
            .map(|source| source.frames)
            .ok_or(CommandError::SourceNotFound(source_id))?;
        if self.source_range.end > source_frames {
            return Err(CommandError::Invalid(
                "trim range is past the source end".into(),
            ));
        }
        let clip = project
            .clip_mut(self.clip_id)
            .ok_or(CommandError::ClipNotFound(self.clip_id))?;
        let new_len = self.source_range.len();
        self.previous = Some((
            std::mem::replace(&mut clip.source_range, self.source_range),
            std::mem::replace(&mut clip.timeline_start, self.timeline_start),
            clip.fade_in_frames,
            clip.fade_out_frames,
        ));
        clip.fade_in_frames = clip.fade_in_frames.min(new_len);
        clip.fade_out_frames = clip.fade_out_frames.min(new_len - clip.fade_in_frames);
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let (range, timeline_start, fade_in, fade_out) = self
            .previous
            .take()
            .ok_or(CommandError::InconsistentState("clip was not trimmed"))?;
        let clip = project
            .clip_mut(self.clip_id)
            .ok_or(CommandError::ClipNotFound(self.clip_id))?;
        clip.source_range = range;
        clip.timeline_start = timeline_start;
        clip.fade_in_frames = fade_in;
        clip.fade_out_frames = fade_out;
        Ok(())
    }
}

/// Split a clip at a frame offset from its start.
#[derive(Debug)]
pub struct SplitClip {
    clip_id: ClipId,
    split_offset: u64,
    right_id: ClipId,
    previous: Option<(FrameRange, u64, u64, Vec<EnvelopePoint>)>,
    right_index: Option<usize>,
}

impl SplitClip {
    /// Build the command. `right_id` is chosen by the caller so the operation is
    /// deterministic and the new clip can be selected immediately.
    #[must_use]
    pub fn new(clip_id: ClipId, split_offset: u64, right_id: ClipId) -> Self {
        Self {
            clip_id,
            split_offset,
            right_id,
            previous: None,
            right_index: None,
        }
    }
}

impl Command for SplitClip {
    fn label(&self) -> &'static str {
        "Split clip"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let (track_index, clip_index) = clip_location(project, self.clip_id)?;
        let original = project.tracks[track_index].clips[clip_index].clone();
        if self.split_offset == 0 || self.split_offset >= original.len() {
            return Err(CommandError::Invalid(
                "split offset must be inside the clip".into(),
            ));
        }
        if project.clip(self.right_id).is_some() {
            return Err(CommandError::Duplicate(format!("clip {}", self.right_id)));
        }

        let split_at = original.source_range.start + self.split_offset;
        let mut left = original.clone();
        let mut right = original.clone();
        left.source_range = FrameRange::new(original.source_range.start, split_at)?;
        right.id = self.right_id;
        right.source_range = FrameRange::new(split_at, original.source_range.end)?;
        right.timeline_start = original.timeline_start + self.split_offset;

        let mut right_envelope = Vec::new();
        left.envelope
            .retain(|point| point.frame <= self.split_offset);
        for point in &original.envelope {
            if point.frame > self.split_offset {
                right_envelope.push(EnvelopePoint {
                    frame: point.frame - self.split_offset,
                    gain: point.gain,
                });
            }
        }
        right.envelope = right_envelope;
        left.fade_in_frames = left.fade_in_frames.min(left.len());
        left.fade_out_frames = 0;
        right.fade_in_frames = 0;
        right.fade_out_frames = right.fade_out_frames.min(right.len());

        self.previous = Some((
            original.source_range,
            original.timeline_start,
            original.fade_out_frames,
            original.envelope,
        ));
        project.tracks[track_index].clips[clip_index] = left;
        let insert_at = clip_index + 1;
        project.tracks[track_index].clips.insert(insert_at, right);
        self.right_index = Some(insert_at);
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let (range, timeline_start, fade_out, envelope) = self
            .previous
            .take()
            .ok_or(CommandError::InconsistentState("clip was not split"))?;
        let right_index = self
            .right_index
            .take()
            .ok_or(CommandError::InconsistentState("split index missing"))?;
        let (track_index, clip_index) = clip_location(project, self.clip_id)?;
        let removed = project.tracks[track_index]
            .clips
            .get(right_index)
            .ok_or(CommandError::ClipNotFound(self.right_id))?;
        if removed.id != self.right_id {
            return Err(CommandError::InconsistentState("split index changed"));
        }
        project.tracks[track_index].clips.remove(right_index);
        let clip = &mut project.tracks[track_index].clips[clip_index];
        clip.source_range = range;
        clip.timeline_start = timeline_start;
        clip.fade_out_frames = fade_out;
        clip.envelope = envelope;
        Ok(())
    }
}

/// Set a clip's linear gain.
#[derive(Debug)]
pub struct SetClipGain {
    clip_id: ClipId,
    gain: f32,
    previous: Option<f32>,
}

impl SetClipGain {
    /// Build the command.
    #[must_use]
    pub fn new(clip_id: ClipId, gain: f32) -> Self {
        Self {
            clip_id,
            gain,
            previous: None,
        }
    }
}

impl Command for SetClipGain {
    fn label(&self) -> &'static str {
        "Set clip gain"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        if !self.gain.is_finite() || self.gain < 0.0 {
            return Err(CommandError::Invalid(
                "gain must be finite and non-negative".into(),
            ));
        }
        let clip = project
            .clip_mut(self.clip_id)
            .ok_or(CommandError::ClipNotFound(self.clip_id))?;
        self.previous = Some(std::mem::replace(&mut clip.gain, self.gain));
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let previous = self
            .previous
            .take()
            .ok_or(CommandError::InconsistentState("clip gain was not set"))?;
        let clip = project
            .clip_mut(self.clip_id)
            .ok_or(CommandError::ClipNotFound(self.clip_id))?;
        clip.gain = previous;
        Ok(())
    }
}

/// Set a clip's fade lengths.
#[derive(Debug)]
pub struct SetClipFades {
    clip_id: ClipId,
    fade_in: u64,
    fade_out: u64,
    previous: Option<(u64, u64)>,
}

impl SetClipFades {
    /// Build the command.
    #[must_use]
    pub fn new(clip_id: ClipId, fade_in: u64, fade_out: u64) -> Self {
        Self {
            clip_id,
            fade_in,
            fade_out,
            previous: None,
        }
    }
}

impl Command for SetClipFades {
    fn label(&self) -> &'static str {
        "Set clip fades"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let clip = project
            .clip_mut(self.clip_id)
            .ok_or(CommandError::ClipNotFound(self.clip_id))?;
        if self.fade_in.saturating_add(self.fade_out) > clip.len() {
            return Err(CommandError::Invalid(
                "fades are longer than the clip".into(),
            ));
        }
        self.previous = Some((
            std::mem::replace(&mut clip.fade_in_frames, self.fade_in),
            std::mem::replace(&mut clip.fade_out_frames, self.fade_out),
        ));
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let (fade_in, fade_out) = self
            .previous
            .take()
            .ok_or(CommandError::InconsistentState("fades were not set"))?;
        let clip = project
            .clip_mut(self.clip_id)
            .ok_or(CommandError::ClipNotFound(self.clip_id))?;
        clip.fade_in_frames = fade_in;
        clip.fade_out_frames = fade_out;
        Ok(())
    }
}

/// Replace a clip's volume envelope.
#[derive(Debug)]
pub struct SetClipEnvelope {
    clip_id: ClipId,
    envelope: Vec<EnvelopePoint>,
    previous: Option<Vec<EnvelopePoint>>,
}

impl SetClipEnvelope {
    /// Build the command.
    #[must_use]
    pub fn new(clip_id: ClipId, envelope: Vec<EnvelopePoint>) -> Self {
        Self {
            clip_id,
            envelope,
            previous: None,
        }
    }
}

impl Command for SetClipEnvelope {
    fn label(&self) -> &'static str {
        "Set clip envelope"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let clip = project
            .clip_mut(self.clip_id)
            .ok_or(CommandError::ClipNotFound(self.clip_id))?;
        let mut previous_frame = 0;
        for (index, point) in self.envelope.iter().enumerate() {
            if !point.gain.is_finite() || point.gain < 0.0 {
                return Err(CommandError::Invalid("envelope gain is invalid".into()));
            }
            if point.frame > clip.len() {
                return Err(CommandError::Invalid(
                    "envelope point is past the clip end".into(),
                ));
            }
            if index > 0 && point.frame < previous_frame {
                return Err(CommandError::Invalid(
                    "envelope points are not sorted".into(),
                ));
            }
            previous_frame = point.frame;
        }
        self.previous = Some(std::mem::replace(&mut clip.envelope, self.envelope.clone()));
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let previous = self
            .previous
            .take()
            .ok_or(CommandError::InconsistentState("envelope was not set"))?;
        let clip = project
            .clip_mut(self.clip_id)
            .ok_or(CommandError::ClipNotFound(self.clip_id))?;
        clip.envelope = previous;
        Ok(())
    }
}

/// Replace a clip's effect chain.
#[derive(Debug)]
pub struct SetClipEffects {
    clip_id: ClipId,
    effects: Vec<EffectRef>,
    previous: Option<Vec<EffectRef>>,
}

impl SetClipEffects {
    /// Build the command.
    #[must_use]
    pub fn new(clip_id: ClipId, effects: Vec<EffectRef>) -> Self {
        Self {
            clip_id,
            effects,
            previous: None,
        }
    }
}

impl Command for SetClipEffects {
    fn label(&self) -> &'static str {
        "Set clip effects"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let clip = project
            .clip_mut(self.clip_id)
            .ok_or(CommandError::ClipNotFound(self.clip_id))?;
        self.previous = Some(std::mem::replace(&mut clip.effects, self.effects.clone()));
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let previous = self
            .previous
            .take()
            .ok_or(CommandError::InconsistentState("effects were not set"))?;
        let clip = project
            .clip_mut(self.clip_id)
            .ok_or(CommandError::ClipNotFound(self.clip_id))?;
        clip.effects = previous;
        Ok(())
    }
}

/// Rename the project.
#[derive(Debug)]
pub struct RenameProject {
    name: String,
    previous: Option<String>,
}

impl RenameProject {
    /// Build the command.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            previous: None,
        }
    }
}

impl Command for RenameProject {
    fn label(&self) -> &'static str {
        "Rename project"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        check_name(&self.name)?;
        self.previous = Some(std::mem::replace(&mut project.name, self.name.clone()));
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let previous = self
            .previous
            .take()
            .ok_or(CommandError::InconsistentState("project was not renamed"))?;
        project.name = previous;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CommandStack, ProjectId};

    fn fixture() -> (Project, TrackId, SourceId) {
        let mut project =
            Project::new_with_id(ProjectId::from_u128(1), "test", 48_000).expect("project");
        let source = Source::new(
            SourceId::from_u128(2),
            "voice.wav",
            "media/voice.wav",
            48_000,
            1,
            48_000,
        )
        .expect("source");
        project.sources.push(source);
        let track = Track::new(TrackId::from_u128(3), "Track 1");
        project.tracks.push(track);
        (project, TrackId::from_u128(3), SourceId::from_u128(2))
    }

    fn fixture_clip(id: u128) -> Clip {
        Clip::new(
            ClipId::from_u128(id),
            SourceId::from_u128(2),
            FrameRange::new(0, 4_800).expect("range"),
            0,
        )
    }

    #[test]
    fn add_and_remove_track_round_trip() {
        let (mut project, _, _) = fixture();
        let initial = serde_json::to_string(&project).expect("json");
        let mut stack = CommandStack::new();
        stack
            .execute(
                &mut project,
                Box::new(AddTrack::new(Track::new(TrackId::from_u128(4), "New"), 1)),
            )
            .expect("add");
        assert_eq!(project.tracks.len(), 2);
        stack.undo(&mut project).expect("undo");
        assert_eq!(serde_json::to_string(&project).expect("json"), initial);
        stack.redo(&mut project).expect("redo");
        assert_eq!(project.tracks.len(), 2);
    }

    #[test]
    fn split_and_revert_restores_clip() {
        let (mut project, track_id, _) = fixture();
        project.tracks[0].clips.push(fixture_clip(10));
        let initial = serde_json::to_string(&project).expect("json");
        let mut stack = CommandStack::new();
        stack
            .execute(
                &mut project,
                Box::new(SplitClip::new(
                    ClipId::from_u128(10),
                    2_400,
                    ClipId::from_u128(11),
                )),
            )
            .expect("split");
        assert_eq!(project.clip_count(), 2);
        project.validate().expect("valid after split");
        stack.undo(&mut project).expect("undo");
        assert_eq!(serde_json::to_string(&project).expect("json"), initial);
        let _ = track_id;
    }

    #[test]
    fn move_clip_between_tracks_and_back() {
        let (mut project, _, _) = fixture();
        project
            .tracks
            .push(Track::new(TrackId::from_u128(4), "Track 2"));
        project.tracks[0].clips.push(fixture_clip(10));
        let initial = serde_json::to_string(&project).expect("json");
        let mut stack = CommandStack::new();
        stack
            .execute(
                &mut project,
                Box::new(MoveClip::new(
                    ClipId::from_u128(10),
                    TrackId::from_u128(4),
                    0,
                    9_600,
                )),
            )
            .expect("move");
        assert_eq!(project.tracks[0].clips.len(), 0);
        assert_eq!(project.tracks[1].clips[0].timeline_start, 9_600);
        stack.undo(&mut project).expect("undo");
        assert_eq!(serde_json::to_string(&project).expect("json"), initial);
    }

    #[test]
    fn new_command_clears_redo() {
        let (mut project, track_id, _) = fixture();
        let mut stack = CommandStack::new();
        stack
            .execute(&mut project, Box::new(RenameTrack::new(track_id, "A")))
            .expect("rename");
        stack.undo(&mut project).expect("undo");
        assert!(stack.can_redo());
        stack
            .execute(&mut project, Box::new(RenameTrack::new(track_id, "B")))
            .expect("rename");
        assert!(!stack.can_redo());
    }

    #[test]
    fn stack_limit_drops_oldest_history() {
        let (mut project, track_id, _) = fixture();
        let mut stack = CommandStack::with_limit(2);
        for name in ["A", "B", "C"] {
            stack
                .execute(&mut project, Box::new(RenameTrack::new(track_id, name)))
                .expect("rename");
        }
        assert_eq!(stack.undo_len(), 2);
        assert_eq!(stack.undo_label(), Some("Rename track"));
    }
}
