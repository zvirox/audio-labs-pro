//! Automation wrappers for offline dubbing operations.
//!
//! Rate fitting and video remux use existing pure/media implementations and
//! never contact the inference daemon. Stem separation stays out of this crate
//! because `alp-api` may not depend on `alp-infer`.

use alp_media::{ExportSettings, RemuxReport, RemuxRequest, VideoRemuxer, WavBitDepth, decode, export};
use alp_voice::{FitPlan, fit_take};
use serde::{Deserialize, Serialize};

use crate::{ApiError, PermissionTier, session::ApiSession};

/// Request to fit a rendered take to a dialogue window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DubbingFitRequestDto {
    /// Rendered take on disk.
    pub audio_path: String,
    /// Target window duration in seconds.
    pub target_duration_secs: f64,
    /// Destination WAV path.
    pub output_path: String,
}

/// Result of a rate-fit request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DubbingFitReport {
    /// Destination WAV path.
    pub path: String,
    /// Resulting duration in seconds.
    pub duration_secs: f64,
    /// Plan used for the fit.
    pub plan: FitPlan,
}

/// Request to remux a video with a dubbed audio track.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DubbingRemuxRequestDto {
    /// Source video (MP4/MKV).
    pub video_path: String,
    /// Dubbed audio track; when omitted the video's own audio is used.
    pub audio_path: Option<String>,
    /// Destination video path.
    pub output_path: String,
    /// Optional subtitle side file (not burned in).
    pub subtitle_path: Option<String>,
}

impl ApiSession {
    /// Fit a rendered take to a target duration with WSOLA rate adjustment.
    pub fn dubbing_fit_rate(
        &mut self,
        request: &DubbingFitRequestDto,
    ) -> Result<DubbingFitReport, ApiError> {
        self.check_permission(PermissionTier::Export)?;
        if !request.target_duration_secs.is_finite() || request.target_duration_secs <= 0.0 {
            return Err(ApiError::Validation(
                "target_duration_secs must be positive and finite".into(),
            ));
        }
        let input = self.sandbox.validate_path(&request.audio_path)?;
        let output = self.sandbox.validate_path(&request.output_path)?;
        let take = decode(&input)?;
        let (fitted, plan) = fit_take(&take, request.target_duration_secs)?;
        export(
            &output,
            &fitted,
            &ExportSettings::wav(WavBitDepth::Int16),
        )?;
        Ok(DubbingFitReport {
            path: output.display().to_string(),
            duration_secs: fitted.duration_secs(),
            plan,
        })
    }

    /// Remux a video with a dubbed audio track without re-encoding video.
    pub fn dubbing_remux(
        &mut self,
        request: &DubbingRemuxRequestDto,
    ) -> Result<RemuxReport, ApiError> {
        self.check_permission(PermissionTier::Export)?;
        let video = self.sandbox.validate_path(&request.video_path)?;
        let output = self.sandbox.validate_path(&request.output_path)?;
        let audio = request
            .audio_path
            .as_deref()
            .map(|path| self.sandbox.validate_path(path))
            .transpose()?;
        let subtitle = request
            .subtitle_path
            .as_deref()
            .map(|path| self.sandbox.validate_path(path))
            .transpose()?;
        let remux_request = RemuxRequest {
            video_path: video.display().to_string(),
            audio_path: audio.map(|path| path.display().to_string()),
            output_path: output.display().to_string(),
            subtitle_path: subtitle.map(|path| path.display().to_string()),
        };
        Ok(VideoRemuxer::new().remux(&remux_request)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuditLogger, WorkspaceSandbox};
    use alp_core::{Project, ProjectPaths};
    use alp_media::write_test_tone;
    use tempfile::tempdir;

    fn session_in(dir: &std::path::Path) -> ApiSession {
        let paths = ProjectPaths::new(dir.join("proj.alp"));
        paths.ensure_layout().expect("layout");
        let sandbox = WorkspaceSandbox::new([dir]).expect("sandbox");
        let audit = AuditLogger::new(dir.join("audit.jsonl")).expect("audit");
        ApiSession::new(
            Project::new("Dubbing Test").expect("project"),
            paths,
            PermissionTier::Export,
            sandbox,
            audit,
            "agent",
        )
        .expect("session")
    }

    #[test]
    fn fit_rate_writes_a_fitted_take() {
        let dir = tempdir().expect("tempdir");
        let input = dir.path().join("take.wav");
        let output = dir.path().join("fitted.wav");
        write_test_tone(&input, 0.2, 48_000, 1, 440.0, 0.2).expect("write take");
        let mut session = session_in(dir.path());
        let report = session
            .dubbing_fit_rate(&DubbingFitRequestDto {
                audio_path: input.display().to_string(),
                target_duration_secs: 0.15,
                output_path: output.display().to_string(),
            })
            .expect("fit rate");
        assert!(output.is_file());
        assert!(report.duration_secs > 0.0);
        assert!(report.plan.target_duration_secs > 0.0);
    }

    #[test]
    fn invalid_target_duration_is_rejected() {
        let dir = tempdir().expect("tempdir");
        let mut session = session_in(dir.path());
        let error = session
            .dubbing_fit_rate(&DubbingFitRequestDto {
                audio_path: "take.wav".into(),
                target_duration_secs: 0.0,
                output_path: "out.wav".into(),
            })
            .expect_err("invalid duration");
        assert!(matches!(error, ApiError::Validation(_)));
    }

    #[test]
    fn remux_reports_a_missing_video_without_panicking() {
        let dir = tempdir().expect("tempdir");
        let mut session = session_in(dir.path());
        let result = session.dubbing_remux(&DubbingRemuxRequestDto {
            video_path: "missing.mp4".into(),
            audio_path: None,
            output_path: "out.mp4".into(),
            subtitle_path: None,
        });
        // ffmpeg/ffprobe availability decides the exact error, never a panic.
        let _ = result;
    }
}
