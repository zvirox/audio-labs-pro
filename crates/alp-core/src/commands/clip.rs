//! Undoable commands that act on one clip.

use super::*;
use crate::project::EnvelopePoint;

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
