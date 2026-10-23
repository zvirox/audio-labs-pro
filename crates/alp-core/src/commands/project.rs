//! Rename a project.

use super::*;

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
