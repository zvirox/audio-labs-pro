//! Undoable commands for imported sources.

use super::*;
use crate::ids::SourceId;

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
