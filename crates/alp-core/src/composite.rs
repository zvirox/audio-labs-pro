//! Group several already-built commands into one undoable edit.
//!
//! A batch edit is assembled from commands that were executed in sequence. It
//! wraps them so the whole group is one undo step: reverting walks the commands
//! in reverse, and re-applying walks them forward. If one command in the group
//! fails to re-apply, the commands already applied are reverted so the project
//! is left exactly as it was before the group ran.

use std::fmt;

use crate::command::{Command, CommandError};
use crate::project::Project;

/// Several commands treated as a single undoable edit.
pub struct CompositeCommand {
    label: &'static str,
    commands: Vec<Box<dyn Command>>,
}

impl CompositeCommand {
    /// Build a composite from commands that were applied in order.
    #[must_use]
    pub fn new(label: &'static str, commands: Vec<Box<dyn Command>>) -> Self {
        Self { label, commands }
    }

    /// Number of commands in the group.
    #[must_use]
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Whether the group holds no commands.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

impl fmt::Debug for CompositeCommand {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CompositeCommand")
            .field("label", &self.label)
            .field("commands", &self.commands)
            .finish()
    }
}

impl Command for CompositeCommand {
    fn label(&self) -> &'static str {
        self.label
    }

    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError> {
        for (applied, command) in self.commands.iter_mut().enumerate() {
            if let Err(error) = command.apply(project) {
                // Leave the project untouched: revert what this group applied.
                for done in self.commands[..applied].iter_mut().rev() {
                    let _ = done.revert(project);
                }
                return Err(error);
            }
        }
        Ok(())
    }

    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError> {
        for command in self.commands.iter_mut().rev() {
            command.revert(project)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{AddTrack, RemoveTrack};
    use crate::ids::TrackId;
    use crate::project::Track;

    #[test]
    fn composite_applies_and_reverts_as_one_step() {
        let mut project = Project::new("Composite").expect("project");
        let first = TrackId::new();
        let second = TrackId::new();
        let mut composite = CompositeCommand::new(
            "Batch",
            vec![
                Box::new(AddTrack::new(Track::new(first, "One"), 0)),
                Box::new(AddTrack::new(Track::new(second, "Two"), 1)),
            ],
        );
        assert_eq!(composite.len(), 2);

        composite.apply(&mut project).expect("apply");
        assert_eq!(project.tracks.len(), 2);

        composite.revert(&mut project).expect("revert");
        assert!(project.tracks.is_empty());
    }

    #[test]
    fn a_failing_command_leaves_the_project_untouched() {
        let mut project = Project::new("Composite").expect("project");
        let track = TrackId::new();
        let missing = TrackId::new();
        let mut composite = CompositeCommand::new(
            "Batch",
            vec![
                Box::new(AddTrack::new(Track::new(track, "Kept"), 0)),
                // Removing a track that does not exist fails the whole group.
                Box::new(RemoveTrack::new(missing)),
            ],
        );

        assert!(composite.apply(&mut project).is_err());
        assert!(
            project.tracks.is_empty(),
            "the first command must be rolled back when a later one fails"
        );
    }
}
