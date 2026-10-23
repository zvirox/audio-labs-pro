//! Undoable commands for timeline annotations.

use super::*;
use crate::ids::{MarkerId, RegionId};
use crate::markers::{Marker, Region};

fn marker_position(project: &Project, id: MarkerId) -> Result<usize, CommandError> {
    project
        .markers
        .iter()
        .position(|marker| marker.id == id)
        .ok_or(CommandError::Invalid(format!("marker {id} not found")))
}

fn label(label: &str) -> Result<(), CommandError> {
    if label.trim().is_empty() {
        return Err(CommandError::Invalid("label is empty".into()));
    }
    Ok(())
}

/// Add a marker at a timeline frame.
#[derive(Debug)]
pub struct AddMarker {
    marker: Marker,
}

impl AddMarker {
    /// Build the command.
    #[must_use]
    pub fn new(id: MarkerId, frame: u64, label: impl Into<String>) -> Self {
        Self {
            marker: Marker::new(id, frame, label),
        }
    }
}

impl Command for AddMarker {
    fn label(&self) -> &'static str {
        "Add marker"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        label(&self.marker.label)?;
        if project.markers.iter().any(|m| m.id == self.marker.id) {
            return Err(CommandError::Duplicate(self.marker.id.to_string()));
        }
        let frame = self.marker.frame;
        let index = project
            .markers
            .partition_point(|marker| marker.frame <= frame);
        project.markers.insert(index, self.marker.clone());
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let index = marker_position(project, self.marker.id)?;
        let marker = project.markers.remove(index);
        debug_assert_eq!(marker.id, self.marker.id);
        self.marker = marker;
        Ok(())
    }
}

/// Rename an existing marker.
#[derive(Debug)]
pub struct RenameMarker {
    id: MarkerId,
    label: String,
    previous: Option<String>,
}

impl RenameMarker {
    /// Build the command.
    #[must_use]
    pub fn new(id: MarkerId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            previous: None,
        }
    }
}

impl Command for RenameMarker {
    fn label(&self) -> &'static str {
        "Rename marker"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        label(&self.label)?;
        let index = marker_position(project, self.id)?;
        let previous = std::mem::replace(&mut project.markers[index].label, self.label.clone());
        self.previous = Some(previous);
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let previous = self
            .previous
            .take()
            .ok_or(CommandError::InconsistentState("marker was not renamed"))?;
        let index = marker_position(project, self.id)?;
        project.markers[index].label = previous;
        Ok(())
    }
}

/// Remove a marker.
#[derive(Debug)]
pub struct RemoveMarker {
    id: MarkerId,
    removed: Option<(usize, Marker)>,
}

impl RemoveMarker {
    /// Build the command.
    #[must_use]
    pub fn new(id: MarkerId) -> Self {
        Self { id, removed: None }
    }
}

impl Command for RemoveMarker {
    fn label(&self) -> &'static str {
        "Remove marker"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let index = marker_position(project, self.id)?;
        let marker = project.markers.remove(index);
        self.removed = Some((index, marker));
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let (index, marker) = self
            .removed
            .take()
            .ok_or(CommandError::InconsistentState("marker was not removed"))?;
        project
            .markers
            .insert(index.min(project.markers.len()), marker);
        Ok(())
    }
}

/// Add a named region.
#[derive(Debug)]
pub struct AddRegion {
    region: Region,
}

impl AddRegion {
    /// Build the command.
    #[must_use]
    pub fn new(id: RegionId, range: FrameRange, label: impl Into<String>) -> Self {
        Self {
            region: Region::new(id, range, label),
        }
    }
}

impl Command for AddRegion {
    fn label(&self) -> &'static str {
        "Add region"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        label(&self.region.label)?;
        if project.regions.iter().any(|r| r.id == self.region.id) {
            return Err(CommandError::Duplicate(self.region.id.to_string()));
        }
        let range = self.region.range;
        let index = project
            .regions
            .partition_point(|region| region.range.start <= range.start);
        project.regions.insert(index, self.region.clone());
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let index = project
            .regions
            .iter()
            .position(|region| region.id == self.region.id)
            .ok_or(CommandError::Invalid(format!(
                "region {} not found",
                self.region.id
            )))?;
        let region = project.regions.remove(index);
        debug_assert_eq!(region.id, self.region.id);
        self.region = region;
        Ok(())
    }
}

/// Rename a region.
#[derive(Debug)]
pub struct RenameRegion {
    id: RegionId,
    label: String,
    previous: Option<String>,
}

impl RenameRegion {
    /// Build the command.
    #[must_use]
    pub fn new(id: RegionId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            previous: None,
        }
    }
}

impl Command for RenameRegion {
    fn label(&self) -> &'static str {
        "Rename region"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        label(&self.label)?;
        let index = project
            .regions
            .iter()
            .position(|region| region.id == self.id)
            .ok_or(CommandError::Invalid(format!(
                "region {} not found",
                self.id
            )))?;
        let previous = std::mem::replace(&mut project.regions[index].label, self.label.clone());
        self.previous = Some(previous);
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let previous = self
            .previous
            .take()
            .ok_or(CommandError::InconsistentState("region was not renamed"))?;
        let index = project
            .regions
            .iter()
            .position(|region| region.id == self.id)
            .ok_or(CommandError::Invalid(format!(
                "region {} not found",
                self.id
            )))?;
        project.regions[index].label = previous;
        Ok(())
    }
}

/// Remove a region.
#[derive(Debug)]
pub struct RemoveRegion {
    id: RegionId,
    removed: Option<(usize, Region)>,
}

impl RemoveRegion {
    /// Build the command.
    #[must_use]
    pub fn new(id: RegionId) -> Self {
        Self { id, removed: None }
    }
}

impl Command for RemoveRegion {
    fn label(&self) -> &'static str {
        "Remove region"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let index = project
            .regions
            .iter()
            .position(|region| region.id == self.id)
            .ok_or(CommandError::Invalid(format!(
                "region {} not found",
                self.id
            )))?;
        let region = project.regions.remove(index);
        self.removed = Some((index, region));
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let (index, region) = self
            .removed
            .take()
            .ok_or(CommandError::InconsistentState("region was not removed"))?;
        project
            .regions
            .insert(index.min(project.regions.len()), region);
        Ok(())
    }
}

/// Set the loop range column carrier (optional range).
#[derive(Debug)]
pub struct SetLoopRange {
    range: Option<FrameRange>,
    previous: Option<Option<FrameRange>>,
}

impl SetLoopRange {
    /// Build the command.
    #[must_use]
    pub fn new(range: Option<FrameRange>) -> Self {
        Self {
            range,
            previous: None,
        }
    }

    /// Build the command from a range.
    #[must_use]
    pub fn from_range(range: FrameRange) -> Self {
        Self::new(Some(range))
    }
}

impl Command for SetLoopRange {
    fn label(&self) -> &'static str {
        "Set loop range"
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        if let Some(range) = self.range
            && range.start >= range.end
        {
            return Err(CommandError::Invalid(
                "loop range start must be before its end".into(),
            ));
        }
        self.previous = Some(project.loop_range);
        project.loop_range = self.range;
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let previous = self.previous.take().ok_or(CommandError::InconsistentState(
            "loop range was not changed",
        ))?;
        project.loop_range = previous;
        Ok(())
    }
}
