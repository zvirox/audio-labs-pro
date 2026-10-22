//! The undo/redo command protocol and stack.
//!
//! Every mutation is represented by a [`Command`] whose `apply` is inverted by
//! `revert`. The stack owns the commands and never touches the project itself.

use std::fmt;

use crate::error::CoreError;
use crate::ids::{ClipId, SourceId, TrackId};
use crate::project::Project;

/// Why a command could not be applied or reverted.
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    /// Track id is not present in the project.
    #[error("track {0} not found")]
    TrackNotFound(TrackId),
    /// Clip id is not present in the project.
    #[error("clip {0} not found")]
    ClipNotFound(ClipId),
    /// Source id is not present in the project.
    #[error("source {0} not found")]
    SourceNotFound(SourceId),
    /// Insertion or removal index is outside the collection.
    #[error("index {index} is out of bounds for length {len}")]
    IndexOutOfBounds {
        /// Requested index.
        index: usize,
        /// Current collection length.
        len: usize,
    },
    /// The command would create a duplicate identifier.
    #[error("duplicate identifier: {0}")]
    Duplicate(String),
    /// The command arguments are not valid for the current state.
    #[error("invalid command: {0}")]
    Invalid(String),
    /// A command tried to revert something it did not apply.
    #[error("command state is inconsistent: {0}")]
    InconsistentState(&'static str),
    /// Project-level validation failed.
    #[error(transparent)]
    Core(#[from] CoreError),
}

/// A reversible mutation of a [`Project`].
pub trait Command: fmt::Debug + Send {
    /// Short human-readable label for history UI and logs.
    fn label(&self) -> &'static str;

    /// Apply the mutation. Must leave the project unchanged on error.
    fn apply(&mut self, project: &mut Project) -> Result<(), CommandError>;

    /// Undo a previous [`Command::apply`].
    fn revert(&mut self, project: &mut Project) -> Result<(), CommandError>;
}

/// Bounded undo/redo history.
#[derive(Debug)]
pub struct CommandStack {
    undo: Vec<Box<dyn Command>>,
    redo: Vec<Box<dyn Command>>,
    limit: usize,
}

impl Default for CommandStack {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandStack {
    /// Default maximum number of undoable steps.
    pub const DEFAULT_LIMIT: usize = 512;

    /// Create an empty stack with the default limit.
    #[must_use]
    pub fn new() -> Self {
        Self::with_limit(Self::DEFAULT_LIMIT)
    }

    /// Create an empty stack with a custom limit (minimum 1).
    #[must_use]
    pub fn with_limit(limit: usize) -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            limit: limit.max(1),
        }
    }

    /// Apply a command and push it onto the undo stack.
    ///
    /// A successful command clears the redo stack. The oldest entries are
    /// discarded when the history exceeds its limit.
    pub fn execute(
        &mut self,
        project: &mut Project,
        mut command: Box<dyn Command>,
    ) -> Result<(), CommandError> {
        command.apply(project)?;
        self.redo.clear();
        self.undo.push(command);
        let excess = self.undo.len().saturating_sub(self.limit);
        if excess > 0 {
            self.undo.drain(0..excess);
        }
        Ok(())
    }

    /// Revert the most recent command.
    ///
    /// Returns `false` when there is nothing to undo.
    pub fn undo(&mut self, project: &mut Project) -> Result<bool, CommandError> {
        let Some(command) = self.undo.last_mut() else {
            return Ok(false);
        };
        command.revert(project)?;
        let command = self.undo.pop().expect("checked above");
        self.redo.push(command);
        Ok(true)
    }

    /// Re-apply the most recently undone command.
    ///
    /// Returns `false` when there is nothing to redo.
    pub fn redo(&mut self, project: &mut Project) -> Result<bool, CommandError> {
        let Some(command) = self.redo.last_mut() else {
            return Ok(false);
        };
        command.apply(project)?;
        let command = self.redo.pop().expect("checked above");
        self.undo.push(command);
        let excess = self.undo.len().saturating_sub(self.limit);
        if excess > 0 {
            self.undo.drain(0..excess);
        }
        Ok(true)
    }

    /// Whether an undo step is available.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Whether a redo step is available.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Number of undo steps.
    #[must_use]
    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    /// Number of redo steps.
    #[must_use]
    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }

    /// Label of the next undo step.
    #[must_use]
    pub fn undo_label(&self) -> Option<&'static str> {
        self.undo.last().map(|command| command.label())
    }

    /// Label of the next redo step.
    #[must_use]
    pub fn redo_label(&self) -> Option<&'static str> {
        self.redo.last().map(|command| command.label())
    }

    /// Drop all history.
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}
