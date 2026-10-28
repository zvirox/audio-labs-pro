//! Undoable commands that act on whole tracks.

use super::*;

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

/// Change a track's display colour.
#[derive(Debug)]
pub struct SetTrackColor {
    track_id: TrackId,
    color: [u8; 3],
    previous: Option<[u8; 3]>,
}

impl SetTrackColor {
    /// Build the command.
    #[must_use]
    pub fn new(track_id: TrackId, color: [u8; 3]) -> Self {
        Self {
            track_id,
            color,
            previous: None,
        }
    }
}

impl Command for SetTrackColor {
    fn label(&self) -> &'static str {
        "Set track colour"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let index = track_index(project, self.track_id)?;
        self.previous = Some(project.tracks[index].color);
        project.tracks[index].color = self.color;
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let previous = self.previous.take().ok_or(CommandError::InconsistentState(
            "track colour was not changed",
        ))?;
        let index = track_index(project, self.track_id)?;
        project.tracks[index].color = previous;
        Ok(())
    }
}

/// Arm or disarm a track for recording.
#[derive(Debug)]
pub struct SetTrackArm {
    track_id: TrackId,
    armed: bool,
    previous: Option<bool>,
}

impl SetTrackArm {
    /// Build the command.
    #[must_use]
    pub fn new(track_id: TrackId, armed: bool) -> Self {
        Self {
            track_id,
            armed,
            previous: None,
        }
    }
}

impl Command for SetTrackArm {
    fn label(&self) -> &'static str {
        "Arm track"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let index = track_index(project, self.track_id)?;
        self.previous = Some(project.tracks[index].armed);
        project.tracks[index].armed = self.armed;
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let previous = self.previous.take().ok_or(CommandError::InconsistentState(
            "track arm state was not changed",
        ))?;
        let index = track_index(project, self.track_id)?;
        project.tracks[index].armed = previous;
        Ok(())
    }
}

/// Replace a track's effect chain.
#[derive(Debug)]
pub struct SetTrackEffects {
    track_id: TrackId,
    effects: Vec<EffectRef>,
    previous: Option<Vec<EffectRef>>,
}

impl SetTrackEffects {
    /// Build the command.
    #[must_use]
    pub fn new(track_id: TrackId, effects: Vec<EffectRef>) -> Self {
        Self {
            track_id,
            effects,
            previous: None,
        }
    }
}

impl Command for SetTrackEffects {
    fn label(&self) -> &'static str {
        "Set track effects"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let index = track_index(project, self.track_id)?;
        self.previous = Some(std::mem::replace(
            &mut project.tracks[index].effects,
            self.effects.clone(),
        ));
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let previous = self.previous.take().ok_or(CommandError::InconsistentState(
            "track effects were not changed",
        ))?;
        let index = track_index(project, self.track_id)?;
        project.tracks[index].effects = previous;
        Ok(())
    }
}
