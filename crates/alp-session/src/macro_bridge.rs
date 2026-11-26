//! Desktop bridge for offline DSP macro batches.
//!
//! Builds a short-lived automation session rooted at the current workspace so
//! the Tauri shell can run a macro batch without depending on `alp-api` or
//! `alp-engine` directly.

use alp_api::{ApiSession, MacroJobReport, MacroRequest, PermissionTier};
use alp_core::{Project, ProjectPaths};

/// Run a DSP macro batch with a fresh Export-tier API session.
pub fn dsp_macro_execute(request: &MacroRequest) -> Result<MacroJobReport, String> {
    let root = std::env::current_dir().map_err(|error| format!("no working directory: {error}"))?;
    let paths = ProjectPaths::new(root.join("macro-batch.alp"));
    let sandbox = alp_api::WorkspaceSandbox::new([root.as_path()])
        .map_err(|error| format!("workspace sandbox failed: {error}"))?;
    let audit = alp_api::AuditLogger::new(root.join("audit.jsonl"))
        .map_err(|error| format!("audit log failed: {error}"))?;
    let project = Project::new("Macro Batch").map_err(|error| error.to_string())?;
    let mut session = ApiSession::new(
        project,
        paths,
        PermissionTier::Export,
        sandbox,
        audit,
        "desktop-macro",
    )
    .map_err(|error| error.to_string())?;
    session
        .dsp_macro_execute(request)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use alp_api::MacroStep;

    use super::*;

    #[test]
    fn empty_steps_are_rejected() {
        let request = MacroRequest {
            input_dir: ".".into(),
            output_dir: "out".into(),
            pattern: None,
            overwrite: false,
            steps: Vec::<MacroStep>::new(),
        };
        assert!(dsp_macro_execute(&request).is_err());
    }
}
