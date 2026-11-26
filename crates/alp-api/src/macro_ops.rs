//! Automation job wrapper for offline DSP macro batches.

use alp_engine::{MacroReport, MacroRequest, process_directory};
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::session::ApiSession;
use crate::types::PermissionTier;

/// Result of a `dsp_macro_execute` call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MacroJobReport {
    /// Job handle for `job_status` / `job_cancel`.
    pub job_id: String,
    /// Per-file batch outcome.
    pub report: MacroReport,
}

impl ApiSession {
    /// Enqueue and run a DSP macro over a directory of audio files.
    ///
    /// Input and output directories are confined to the workspace sandbox.
    pub fn dsp_macro_execute(
        &mut self,
        request: &MacroRequest,
    ) -> Result<MacroJobReport, ApiError> {
        self.check_permission(PermissionTier::Export)?;
        if request.steps.is_empty() {
            return Err(ApiError::Validation(
                "steps is empty; add at least one DSP effect".into(),
            ));
        }
        let input = self.sandbox.validate_path(&request.input_dir)?;
        let output = self.sandbox.validate_path(&request.output_dir)?;
        let scoped = MacroRequest {
            input_dir: input.display().to_string(),
            output_dir: output.display().to_string(),
            ..request.clone()
        };

        let job_id = self.jobs.submit("dsp macro execute");
        let result = {
            let jobs = &self.jobs;
            let id = job_id.clone();
            process_directory(&scoped, |completed, total, _path| {
                if jobs.is_cancelled(&id) {
                    return false;
                }
                let progress = if total > 0 {
                    completed as f32 / total as f32
                } else {
                    0.0
                };
                jobs.update_progress(&id, progress, format!("macro file {completed}/{total}"));
                true
            })
        };

        match result {
            Ok(report) => {
                self.jobs.complete(
                    &job_id,
                    format!("{} succeeded, {} failed", report.succeeded, report.failed),
                );
                self.record_audit("dsp_macro_execute", "ok", self.revision());
                Ok(MacroJobReport { job_id, report })
            }
            Err(error) => {
                let message = error.to_string();
                self.jobs.fail(&job_id, message.clone());
                Err(ApiError::Validation(message))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::AuditLogger;
    use crate::sandbox::WorkspaceSandbox;
    use alp_core::{Project, ProjectPaths};
    use alp_engine::MacroStep;
    use alp_media::{ExportSettings, PcmBuffer, WavBitDepth, export};
    use std::collections::BTreeMap;
    use std::f32::consts::PI;
    use tempfile::tempdir;

    fn session_in(dir: &std::path::Path) -> ApiSession {
        let proj_paths = ProjectPaths::new(dir.join("proj.alp"));
        proj_paths.ensure_layout().expect("layout");
        let project = Project::new("Macro Test").expect("project");
        let sandbox = WorkspaceSandbox::new([dir]).expect("sandbox");
        let audit = AuditLogger::new(dir.join("audit.jsonl")).expect("audit");
        ApiSession::new(
            project,
            proj_paths,
            PermissionTier::Export,
            sandbox,
            audit,
            "agent",
        )
        .expect("session")
    }

    #[test]
    fn macro_job_runs_and_reports_files() {
        let dir = tempdir().expect("tempdir");
        let input = dir.path().join("in");
        let output = dir.path().join("out");
        std::fs::create_dir_all(&input).expect("input dir");
        let samples: Vec<f32> = (0..4_800)
            .map(|i| 0.2 * (2.0 * PI * 220.0 * i as f32 / 48_000.0).sin())
            .collect();
        let pcm = PcmBuffer::new(48_000, 1, samples).expect("pcm");
        export(
            input.join("take.wav"),
            &pcm,
            &ExportSettings::wav(WavBitDepth::Int16),
        )
        .expect("write take");

        let mut session = session_in(dir.path());
        let request = MacroRequest {
            input_dir: input.display().to_string(),
            output_dir: output.display().to_string(),
            pattern: None,
            overwrite: true,
            steps: vec![MacroStep {
                effect_id: "gain".into(),
                params: BTreeMap::from([("gain".into(), 0.5)]),
            }],
        };
        let result = session.dsp_macro_execute(&request).expect("macro job");
        assert_eq!(result.report.succeeded, 1);
        assert!(output.join("take.wav").is_file());
        let status = session.job_status(&result.job_id).expect("job status");
        assert_eq!(status.state, "completed");
    }

    #[test]
    fn empty_steps_are_rejected() {
        let dir = tempdir().expect("tempdir");
        let mut session = session_in(dir.path());
        let request = MacroRequest {
            input_dir: dir.path().display().to_string(),
            output_dir: dir.path().join("out").display().to_string(),
            pattern: None,
            overwrite: true,
            steps: Vec::new(),
        };
        assert!(session.dsp_macro_execute(&request).is_err());
    }
}
