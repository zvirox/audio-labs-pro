//! Versioned JSON envelope for `project.json` and the migration scaffold.
//!
//! The version lives beside the project payload so a future schema change can
//! transform the JSON before it is deserialized into the current model.

use serde::Serialize;
use serde_json::Value;

use crate::CoreError;
use crate::project::Project;

/// Newest schema version this build can read and write.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Serialize)]
struct ProjectEnvelope<'a> {
    schema_version: u32,
    project: &'a Project,
}

/// Serialize a project with the current schema version.
pub fn to_json(project: &Project) -> Result<String, CoreError> {
    project.validate()?;
    let envelope = ProjectEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        project,
    };
    Ok(serde_json::to_string_pretty(&envelope)?)
}

/// Deserialize a project, applying migrations when needed.
pub fn from_json(text: &str) -> Result<Project, CoreError> {
    let value: Value = serde_json::from_str(text)?;
    let value = migrate(value)?;
    let project: Project = serde_json::from_value(
        value
            .get("project")
            .cloned()
            .ok_or_else(|| CoreError::InvalidProjectFile("missing project object".into()))?,
    )?;
    project.validate()?;
    Ok(project)
}

/// Schema version declared by a JSON document.
pub fn schema_version(value: &Value) -> Result<u32, CoreError> {
    value
        .get("schema_version")
        .and_then(Value::as_u64)
        .and_then(|version| u32::try_from(version).ok())
        .ok_or(CoreError::MissingSchemaVersion)
}

/// Bring a document up to [`CURRENT_SCHEMA_VERSION`].
///
/// There are no historical versions yet. Future migrations are added as
/// sequential `if version == N { ... }` steps, each bumping `version`, before
/// the final assignment.
pub fn migrate(mut value: Value) -> Result<Value, CoreError> {
    let version = schema_version(&value)?;
    if version == 0 {
        return Err(CoreError::MissingSchemaVersion);
    }
    if version > CURRENT_SCHEMA_VERSION {
        return Err(CoreError::UnsupportedSchemaVersion {
            found: version,
            supported: CURRENT_SCHEMA_VERSION,
        });
    }

    // Add one arm per historical version here, transforming `value` and bumping
    // the local version, before reaching the current version.
    match version {
        CURRENT_SCHEMA_VERSION => {}
        other => {
            return Err(CoreError::InvalidProjectFile(format!(
                "no migration from schema version {other}"
            )));
        }
    }

    value["schema_version"] = Value::from(CURRENT_SCHEMA_VERSION);
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Project;

    #[test]
    fn round_trip_keeps_project() {
        let project = Project::new("round trip").expect("project");
        let text = to_json(&project).expect("serialize");
        let back = from_json(&text).expect("deserialize");
        assert_eq!(project, back);
    }

    #[test]
    fn future_version_is_rejected() {
        let project = Project::new("future").expect("project");
        let mut value: Value =
            serde_json::from_str(&to_json(&project).expect("serialize")).expect("json");
        value["schema_version"] = Value::from(CURRENT_SCHEMA_VERSION + 1);
        assert!(matches!(
            migrate(value),
            Err(CoreError::UnsupportedSchemaVersion { .. })
        ));
    }

    #[test]
    fn missing_version_is_rejected() {
        let value = serde_json::json!({ "project": {} });
        assert!(matches!(
            migrate(value),
            Err(CoreError::MissingSchemaVersion)
        ));
    }
}
