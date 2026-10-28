//! Undoable commands for the project model.
//!
//! This file holds the shared helpers and re-exports. The actual command types
//! live in submodules of `commands`: `commands::track`, `commands::clip`,
//! `commands::source`, `commands::markers` and `commands::project`.

use crate::command::{Command, CommandError};
use crate::ids::{ClipId, TrackId};
use crate::project::{Clip, EffectRef, Project, Source, Track};
use crate::time::FrameRange;

mod clip;
mod markers;
mod project;
mod source;
#[cfg(test)]
mod tests;
mod track;

/// Locate a track by id and return its index.
fn track_index(project: &Project, id: TrackId) -> Result<usize, CommandError> {
    project
        .tracks
        .iter()
        .position(|track| track.id == id)
        .ok_or(CommandError::TrackNotFound(id))
}

/// Locate a clip in the project and return its track and clip index.
fn clip_location(project: &Project, id: ClipId) -> Result<(usize, usize), CommandError> {
    for (track_index, track) in project.tracks.iter().enumerate() {
        if let Some(clip_index) = track.clips.iter().position(|clip| clip.id == id) {
            return Ok((track_index, clip_index));
        }
    }
    Err(CommandError::ClipNotFound(id))
}

/// Check an insertion/removal index against a collection length.
fn check_index(index: usize, len: usize) -> Result<(), CommandError> {
    if index > len {
        return Err(CommandError::IndexOutOfBounds { index, len });
    }
    Ok(())
}

/// Require a non-empty display name.
fn check_name(name: &str) -> Result<(), CommandError> {
    if name.trim().is_empty() {
        return Err(CommandError::Invalid("name is empty".into()));
    }
    Ok(())
}

pub use clip::{
    AddClip, MoveClip, RemoveClip, SetClipEffects, SetClipEnvelope, SetClipFades, SetClipGain,
    SplitClip, TrimClip,
};
pub use markers::{
    AddMarker, AddRegion, RemoveMarker, RemoveRegion, RenameMarker, RenameRegion, SetLoopRange,
};
pub use project::RenameProject;
pub use source::{AddSource, RemoveSource};
pub use track::{
    AddTrack, MoveTrack, RemoveTrack, RenameTrack, SetTrackArm, SetTrackColor, SetTrackEffects,
    SetTrackGain, SetTrackMuted, SetTrackPan, SetTrackSolo,
};
