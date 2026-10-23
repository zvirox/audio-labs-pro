//! Phase 3 range edits: delete, silence, paste, duplicate, punch-in and clip
//! processing.
//!
//! Range commands compute the complete replacement clip list once and remember
//! it for redo. That keeps generated split ids stable across undo/redo and makes
//! every edit a single history step.

use crate::clipboard::{ClipFragment, slice_clip};
use crate::command::{Command, CommandError};
use crate::ids::{ClipId, TrackId};
use crate::project::{Clip, Project};
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

fn check_range(range: FrameRange) -> Result<(), CommandError> {
    if range.is_empty() {
        return Err(CommandError::Invalid(
            "range must contain at least one frame".into(),
        ));
    }
    Ok(())
}

/// Remove a timeline range from one track, splitting clips at both boundaries.
///
/// With `ripple` enabled, clips that used to start at or after the range move
/// left by the deleted length. With it disabled the gap stays in place, which
/// is the "silence" behaviour.
#[derive(Debug)]
pub struct DeleteRange {
    track_id: TrackId,
    range: FrameRange,
    ripple: bool,
    previous: Option<Vec<Clip>>,
    after: Option<Vec<Clip>>,
}

impl DeleteRange {
    /// Build a range deletion.
    #[must_use]
    pub fn new(track_id: TrackId, range: FrameRange, ripple: bool) -> Self {
        Self {
            track_id,
            range,
            ripple,
            previous: None,
            after: None,
        }
    }
}

impl Command for DeleteRange {
    fn label(&self) -> &'static str {
        "Delete range"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        check_range(self.range)?;
        let index = track_index(project, self.track_id)?;
        if self.after.is_none() {
            let previous = project.tracks[index].clips.clone();
            let after = clear_range(project, index, self.range, self.ripple);
            self.previous = Some(previous);
            self.after = Some(after);
        }
        project.tracks[index].clips = self.after.clone().unwrap_or_default();
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let previous = self
            .previous
            .clone()
            .ok_or(CommandError::InconsistentState("range was not deleted"))?;
        let index = track_index(project, self.track_id)?;
        project.tracks[index].clips = previous;
        Ok(())
    }
}

/// Replace a range with empty timeline space while keeping later clips in place.
#[derive(Debug)]
pub struct SilenceRange {
    inner: DeleteRange,
}

impl SilenceRange {
    /// Build a silence command.
    #[must_use]
    pub fn new(track_id: TrackId, range: FrameRange) -> Self {
        Self {
            inner: DeleteRange::new(track_id, range, false),
        }
    }
}

impl Command for SilenceRange {
    fn label(&self) -> &'static str {
        "Silence range"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        self.inner.apply(project)
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        self.inner.revert(project)
    }
}

/// Paste clipboard fragments at a timeline position.
#[derive(Debug)]
pub struct PasteClips {
    track_id: TrackId,
    at: u64,
    clips: Vec<Clip>,
    previous_len: Option<usize>,
}

impl PasteClips {
    /// Build a paste command. The caller supplies fragments copied by
    /// [`crate::copy_range`]; fresh clip ids are allocated here.
    #[must_use]
    pub fn new(track_id: TrackId, at: u64, fragments: &[ClipFragment]) -> Self {
        let mut clips: Vec<Clip> = fragments
            .iter()
            .map(|fragment| fragment.to_clip(ClipId::new(), at.saturating_add(fragment.offset)))
            .collect();
        clips.sort_by_key(|clip| (clip.timeline_start, clip.id));
        Self {
            track_id,
            at,
            clips,
            previous_len: None,
        }
    }

    /// Timeline position used by the paste.
    #[must_use]
    pub fn at(&self) -> u64 {
        self.at
    }
}

impl Command for PasteClips {
    fn label(&self) -> &'static str {
        "Paste"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        if self.clips.is_empty() {
            return Err(CommandError::Invalid("clipboard is empty".into()));
        }
        let index = track_index(project, self.track_id)?;
        for clip in &self.clips {
            if project.clip(clip.id).is_some() {
                return Err(CommandError::Duplicate(format!("clip {}", clip.id)));
            }
            validate_fragment_clip(project, clip)?;
        }
        let previous_len = *self
            .previous_len
            .get_or_insert(project.tracks[index].clips.len());
        if project.tracks[index].clips.len() != previous_len {
            return Err(CommandError::InconsistentState(
                "track changed between paste and redo",
            ));
        }
        project.tracks[index].clips.extend(self.clips.clone());
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let previous_len = self
            .previous_len
            .ok_or(CommandError::InconsistentState("clipboard was not pasted"))?;
        let index = track_index(project, self.track_id)?;
        if project.tracks[index].clips.len() < previous_len {
            return Err(CommandError::InconsistentState(
                "pasted clip list is too short",
            ));
        }
        project.tracks[index].clips.truncate(previous_len);
        Ok(())
    }
}

/// Duplicate a clip on the same track with a fresh id and a time offset.
#[derive(Debug)]
pub struct DuplicateClip {
    clip_id: ClipId,
    new_id: ClipId,
    offset: u64,
    inserted: Option<(usize, usize)>,
}

impl DuplicateClip {
    /// Build a duplicate command.
    #[must_use]
    pub fn new(clip_id: ClipId, new_id: ClipId, offset: u64) -> Self {
        Self {
            clip_id,
            new_id,
            offset,
            inserted: None,
        }
    }
}

impl Command for DuplicateClip {
    fn label(&self) -> &'static str {
        "Duplicate clip"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        if project.clip(self.new_id).is_some() {
            return Err(CommandError::Duplicate(format!("clip {}", self.new_id)));
        }
        let (track_index, clip_index) = clip_location(project, self.clip_id)?;
        let mut clip = project.tracks[track_index].clips[clip_index].clone();
        clip.id = self.new_id;
        clip.timeline_start = clip.timeline_start.saturating_add(self.offset);
        let insert_at = clip_index + 1;
        project.tracks[track_index].clips.insert(insert_at, clip);
        self.inserted = Some((track_index, insert_at));
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let (track_index, insert_at) = self
            .inserted
            .ok_or(CommandError::InconsistentState("clip was not duplicated"))?;
        let track = project
            .tracks
            .get_mut(track_index)
            .ok_or(CommandError::InconsistentState("track index changed"))?;
        let removed = track
            .clips
            .get(insert_at)
            .ok_or(CommandError::ClipNotFound(self.new_id))?;
        if removed.id != self.new_id {
            return Err(CommandError::InconsistentState("duplicate index changed"));
        }
        track.clips.remove(insert_at);
        Ok(())
    }
}

/// Set whether a clip plays backwards.
#[derive(Debug)]
pub struct SetClipReversed {
    clip_id: ClipId,
    reversed: bool,
    previous: Option<bool>,
}

impl SetClipReversed {
    /// Build the command.
    #[must_use]
    pub fn new(clip_id: ClipId, reversed: bool) -> Self {
        Self {
            clip_id,
            reversed,
            previous: None,
        }
    }
}

impl Command for SetClipReversed {
    fn label(&self) -> &'static str {
        "Reverse clip"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let clip = project
            .clip_mut(self.clip_id)
            .ok_or(CommandError::ClipNotFound(self.clip_id))?;
        self.previous = Some(std::mem::replace(&mut clip.reversed, self.reversed));
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let previous = self
            .previous
            .take()
            .ok_or(CommandError::InconsistentState("reverse was not set"))?;
        let clip = project
            .clip_mut(self.clip_id)
            .ok_or(CommandError::ClipNotFound(self.clip_id))?;
        clip.reversed = previous;
        Ok(())
    }
}

/// Set whether a clip's samples are phase-inverted.
#[derive(Debug)]
pub struct SetClipInverted {
    clip_id: ClipId,
    inverted: bool,
    previous: Option<bool>,
}

impl SetClipInverted {
    /// Build the command.
    #[must_use]
    pub fn new(clip_id: ClipId, inverted: bool) -> Self {
        Self {
            clip_id,
            inverted,
            previous: None,
        }
    }
}

impl Command for SetClipInverted {
    fn label(&self) -> &'static str {
        "Invert clip"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let clip = project
            .clip_mut(self.clip_id)
            .ok_or(CommandError::ClipNotFound(self.clip_id))?;
        self.previous = Some(std::mem::replace(&mut clip.inverted, self.inverted));
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let previous = self
            .previous
            .take()
            .ok_or(CommandError::InconsistentState("invert was not set"))?;
        let clip = project
            .clip_mut(self.clip_id)
            .ok_or(CommandError::ClipNotFound(self.clip_id))?;
        clip.inverted = previous;
        Ok(())
    }
}

/// Normalize a clip so its source peak reaches `target_peak`.
///
/// The source peak is recorded at import time. Normalizing an unanalysed source
/// (peak zero) fails instead of guessing.
#[derive(Debug)]
pub struct NormalizeClip {
    clip_id: ClipId,
    target_peak: f32,
    previous: Option<f32>,
}

impl NormalizeClip {
    /// Build a normalize command.
    #[must_use]
    pub fn new(clip_id: ClipId, target_peak: f32) -> Self {
        Self {
            clip_id,
            target_peak,
            previous: None,
        }
    }
}

impl Command for NormalizeClip {
    fn label(&self) -> &'static str {
        "Normalize clip"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        if !self.target_peak.is_finite() || self.target_peak <= 0.0 || self.target_peak > 1.0 {
            return Err(CommandError::Invalid(
                "normalize target must be in 0.0..=1.0".into(),
            ));
        }
        let source_id = project
            .clip(self.clip_id)
            .map(|(_, clip)| clip.source_id)
            .ok_or(CommandError::ClipNotFound(self.clip_id))?;
        let peak = project
            .source(source_id)
            .map(|source| source.peak)
            .ok_or(CommandError::SourceNotFound(source_id))?;
        if !peak.is_finite() || peak <= 0.0 {
            return Err(CommandError::Invalid(
                "source has no measured peak; analyse it before normalizing".into(),
            ));
        }
        let gain = (self.target_peak / peak).min(f32::MAX);
        let clip = project
            .clip_mut(self.clip_id)
            .ok_or(CommandError::ClipNotFound(self.clip_id))?;
        self.previous = Some(std::mem::replace(&mut clip.gain, gain));
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let previous = self.previous.take().ok_or(CommandError::InconsistentState(
            "clip gain was not normalized",
        ))?;
        let clip = project
            .clip_mut(self.clip_id)
            .ok_or(CommandError::ClipNotFound(self.clip_id))?;
        clip.gain = previous;
        Ok(())
    }
}

/// Replace a range with freshly recorded fragments as one undoable punch-in.
#[derive(Debug)]
pub struct PunchIn {
    track_id: TrackId,
    range: FrameRange,
    clips: Vec<Clip>,
    previous: Option<Vec<Clip>>,
    after: Option<Vec<Clip>>,
}

impl PunchIn {
    /// Build a punch-in from recorded fragments.
    #[must_use]
    pub fn new(track_id: TrackId, range: FrameRange, fragments: &[ClipFragment]) -> Self {
        let clips = fragments
            .iter()
            .map(|fragment| {
                fragment.to_clip(ClipId::new(), range.start.saturating_add(fragment.offset))
            })
            .collect();
        Self {
            track_id,
            range,
            clips,
            previous: None,
            after: None,
        }
    }
}

impl Command for PunchIn {
    fn label(&self) -> &'static str {
        "Punch in"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        check_range(self.range)?;
        let index = track_index(project, self.track_id)?;
        if self.after.is_none() {
            let previous = project.tracks[index].clips.clone();
            for clip in &self.clips {
                validate_fragment_clip(project, clip)?;
            }
            let mut after = clear_range(project, index, self.range, false);
            after.extend(self.clips.clone());
            after.sort_by_key(|clip| (clip.timeline_start, clip.id));
            self.previous = Some(previous);
            self.after = Some(after);
        }
        project.tracks[index].clips = self.after.clone().unwrap_or_default();
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let previous = self
            .previous
            .clone()
            .ok_or(CommandError::InconsistentState("punch-in was not applied"))?;
        let index = track_index(project, self.track_id)?;
        project.tracks[index].clips = previous;
        Ok(())
    }
}

fn clear_range(
    project: &Project,
    track_index: usize,
    range: FrameRange,
    ripple: bool,
) -> Vec<Clip> {
    let mut result = Vec::new();
    let deleted = range.len();
    for clip in &project.tracks[track_index].clips {
        let clip_end = project.clip_timeline_end(clip);
        let overlaps = clip.timeline_start < range.end && clip_end > range.start;
        if !overlaps {
            let mut kept = clip.clone();
            if ripple && kept.timeline_start >= range.end {
                kept.timeline_start = kept.timeline_start.saturating_sub(deleted);
            }
            result.push(kept);
            continue;
        }

        if clip.timeline_start < range.start
            && let Some(left) = slice_clip(project, clip, clip.timeline_start, range.start, clip.id)
        {
            result.push(left);
        }
        if clip_end > range.end {
            let from = clip.timeline_start.max(range.end);
            if let Some(mut right) = slice_clip(project, clip, from, clip_end, ClipId::new()) {
                if ripple {
                    right.timeline_start = range.start;
                }
                result.push(right);
            }
        }
    }
    result.sort_by_key(|clip| (clip.timeline_start, clip.id));
    result
}

fn validate_fragment_clip(project: &Project, clip: &Clip) -> Result<(), CommandError> {
    if clip.source_range.is_empty() || clip.source_range.end < clip.source_range.start {
        return Err(CommandError::Invalid("fragment range is empty".into()));
    }
    let source = project
        .source(clip.source_id)
        .ok_or(CommandError::SourceNotFound(clip.source_id))?;
    if clip.source_range.end > source.frames {
        return Err(CommandError::Invalid(
            "fragment reads past the source end".into(),
        ));
    }
    if clip.fade_in_frames.saturating_add(clip.fade_out_frames) > clip.len() {
        return Err(CommandError::Invalid(
            "fragment fades are longer than the clip".into(),
        ));
    }
    Ok(())
}

/// Set the master bus gain.
#[derive(Debug)]
pub struct SetMasterGain {
    gain: f32,
    previous: Option<f32>,
}

impl SetMasterGain {
    /// Build the command.
    #[must_use]
    pub fn new(gain: f32) -> Self {
        Self {
            gain,
            previous: None,
        }
    }
}

impl Command for SetMasterGain {
    fn label(&self) -> &'static str {
        "Set master gain"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        if !self.gain.is_finite() || self.gain < 0.0 {
            return Err(CommandError::Invalid(
                "master gain must be finite and non-negative".into(),
            ));
        }
        self.previous = Some(std::mem::replace(&mut project.master_gain, self.gain));
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let previous = self.previous.take().ok_or(CommandError::InconsistentState(
            "master gain was not changed",
        ))?;
        project.master_gain = previous;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CommandStack, EnvelopePoint, Project, ProjectId, Source, SourceId, Track};

    fn fixture() -> (Project, TrackId, ClipId) {
        let mut project =
            Project::new_with_id(ProjectId::from_u128(1), "edits", 48_000).expect("project");
        project.sources.push(
            Source::new(
                SourceId::from_u128(2),
                "voice.wav",
                "media/voice.wav",
                48_000,
                1,
                96_000,
            )
            .expect("source")
            .with_peak(0.25),
        );
        let mut track = Track::new(TrackId::from_u128(3), "Voice");
        track.clips.push(Clip::new(
            ClipId::from_u128(4),
            SourceId::from_u128(2),
            FrameRange::new(0, 48_000).expect("range"),
            0,
        ));
        track.clips.push(Clip::new(
            ClipId::from_u128(5),
            SourceId::from_u128(2),
            FrameRange::new(1_000, 10_000).expect("range"),
            60_000,
        ));
        project.tracks.push(track);
        (project, TrackId::from_u128(3), ClipId::from_u128(4))
    }

    #[test]
    fn delete_splits_and_ripples_later_clips() {
        let (mut project, track, _) = fixture();
        let mut stack = CommandStack::new();
        stack
            .execute(
                &mut project,
                Box::new(DeleteRange::new(
                    track,
                    FrameRange::new(10_000, 20_000).expect("range"),
                    true,
                )),
            )
            .expect("delete");
        assert_eq!(project.tracks[0].clips.len(), 3);
        assert_eq!(project.tracks[0].clips[0].source_range.end, 10_000);
        assert_eq!(project.tracks[0].clips[1].source_range.start, 20_000);
        assert_eq!(project.tracks[0].clips[1].timeline_start, 10_000);
        assert_eq!(project.tracks[0].clips[2].timeline_start, 50_000);
        project.validate().expect("valid");
        stack.undo(&mut project).expect("undo");
        assert_eq!(project.tracks[0].clips.len(), 2);
        assert_eq!(project.tracks[0].clips[1].timeline_start, 60_000);
    }

    #[test]
    fn silence_keeps_following_clips_in_place() {
        let (mut project, track, _) = fixture();
        let mut stack = CommandStack::new();
        stack
            .execute(
                &mut project,
                Box::new(SilenceRange::new(
                    track,
                    FrameRange::new(10_000, 20_000).expect("range"),
                )),
            )
            .expect("silence");
        assert_eq!(project.tracks[0].clips[2].timeline_start, 60_000);
        project.validate().expect("valid");
    }

    #[test]
    fn copied_range_pastes_with_relative_offsets() {
        let (mut project, track, _) = fixture();
        let clipboard = crate::copy_range(
            &project,
            track,
            FrameRange::new(40_000, 65_000).expect("range"),
        )
        .expect("copy");
        let mut stack = CommandStack::new();
        stack
            .execute(
                &mut project,
                Box::new(PasteClips::new(track, 100_000, clipboard.fragments())),
            )
            .expect("paste");
        let pasted = project.tracks[0]
            .clips
            .iter()
            .filter(|clip| clip.timeline_start >= 100_000)
            .collect::<Vec<_>>();
        assert_eq!(pasted.len(), 2);
        assert_eq!(pasted[0].timeline_start, 100_000);
        assert_eq!(pasted[1].timeline_start, 120_000);
        project.validate().expect("valid");
        stack.undo(&mut project).expect("undo");
        assert_eq!(project.tracks[0].clips.len(), 2);
    }

    #[test]
    fn duplicate_uses_a_new_id_and_offset() {
        let (mut project, _, clip) = fixture();
        let mut stack = CommandStack::new();
        stack
            .execute(
                &mut project,
                Box::new(DuplicateClip::new(clip, ClipId::from_u128(99), 1_000)),
            )
            .expect("duplicate");
        assert!(project.clip(ClipId::from_u128(99)).is_some());
        assert_eq!(project.tracks[0].clips.len(), 3);
        stack.undo(&mut project).expect("undo");
        assert!(project.clip(ClipId::from_u128(99)).is_none());
    }

    #[test]
    fn reverse_invert_and_normalize_are_undoable() {
        let (mut project, _, clip_id) = fixture();
        let mut stack = CommandStack::new();
        stack
            .execute(&mut project, Box::new(SetClipReversed::new(clip_id, true)))
            .expect("reverse");
        stack
            .execute(&mut project, Box::new(SetClipInverted::new(clip_id, true)))
            .expect("invert");
        stack
            .execute(&mut project, Box::new(NormalizeClip::new(clip_id, 0.5)))
            .expect("normalize");
        let clip = project.clip(clip_id).expect("clip").1;
        assert!(clip.reversed);
        assert!(clip.inverted);
        assert!((clip.gain - 2.0).abs() < 1e-6);
        project.validate().expect("valid");
        while stack.undo(&mut project).expect("undo") {}
        assert_eq!(project.tracks[0].clips.len(), 2);
        assert!(!project.tracks[0].clips[0].reversed);
        assert!(!project.tracks[0].clips[0].inverted);
        assert!((project.tracks[0].clips[0].gain - 1.0).abs() < 1e-6);
    }

    #[test]
    fn punch_in_replaces_the_range_in_one_step() {
        let (mut project, track, _) = fixture();
        let fragment = ClipFragment {
            source_id: SourceId::from_u128(2),
            source_range: FrameRange::new(0, 10_000).expect("range"),
            offset: 0,
            gain: 1.0,
            fade_in_frames: 0,
            fade_out_frames: 0,
            envelope: vec![EnvelopePoint {
                frame: 0,
                gain: 1.0,
            }],
            reversed: false,
            inverted: false,
        };
        let mut stack = CommandStack::new();
        stack
            .execute(
                &mut project,
                Box::new(PunchIn::new(
                    track,
                    FrameRange::new(10_000, 20_000).expect("range"),
                    &[fragment],
                )),
            )
            .expect("punch in");
        assert!(project.tracks[0].clips.iter().any(|clip| {
            clip.timeline_start == 10_000
                && clip.source_range == FrameRange::new(0, 10_000).unwrap()
        }));
        project.validate().expect("valid");
        stack.undo(&mut project).expect("undo");
        assert_eq!(project.tracks[0].clips.len(), 2);
    }

    #[test]
    fn normalize_requires_a_measured_peak() {
        let (mut project, _, clip) = fixture();
        project.sources[0].peak = 0.0;
        let mut stack = CommandStack::new();
        assert!(
            stack
                .execute(&mut project, Box::new(NormalizeClip::new(clip, 0.5)))
                .is_err()
        );
    }
}
