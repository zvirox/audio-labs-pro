//! Automation wrappers for offline dubbing operations.
//!
//! Rate fitting and video remux use existing pure/media implementations, while
//! stem separation reaches `alp-infer` across the `alp_voice::InferClient` IPC
//! boundary so `alp-api` never depends on `alp-infer`.

use alp_media::{
    ExportSettings, RemuxReport, RemuxRequest, VideoRemuxer, WavBitDepth, decode, export,
};
use alp_voice::{FitPlan, InferClient, fit_take};
use serde::{Deserialize, Serialize};

use crate::{ApiError, PermissionTier, session::ApiSession};

/// Stem names in the order returned by the inference daemon.
pub const API_DUB_STEM_NAMES: [&str; 4] = ["vocals", "drums", "bass", "other"];

/// One exported stem file from `dubbing_separate_stems`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiStemFileDto {
    /// `vocals`, `drums`, `bass`, or `other`.
    pub name: String,
    /// Exported WAV path inside the workspace.
    pub path: String,
}

/// Request to separate a mixed audio file into four stems via `alp-infer`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DubbingStemsRequestDto {
    /// Mixed audio file on disk inside the workspace.
    pub audio_path: String,
    /// Directory inside the workspace that receives `{vocals,drums,bass,other}.wav`.
    pub output_dir: String,
}

/// Result of a four-stem separation call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DubbingStemsReport {
    /// Exported stems in `API_DUB_STEM_NAMES` order.
    pub stems: Vec<ApiStemFileDto>,
    /// Sample rate of the exported stems.
    pub sample_rate: u32,
}

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
        export(&output, &fitted, &ExportSettings::wav(WavBitDepth::Int16))?;
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

    /// Separate a mixed audio file into four WAV stems via the `alp-infer` daemon.
    pub fn dubbing_separate_stems(
        &mut self,
        request: &DubbingStemsRequestDto,
    ) -> Result<DubbingStemsReport, ApiError> {
        self.check_permission(PermissionTier::Export)?;
        let input = self.sandbox.validate_path(&request.audio_path)?;
        let output_dir = self.sandbox.validate_path(&request.output_dir)?;
        let mixed = decode(&input)?;
        let client = InferClient::new();
        if !client.is_alive() {
            return Err(ApiError::Validation(
                "alp-infer daemon is not running; start it before separating stems".into(),
            ));
        }
        let status = client
            .status()
            .map_err(|error| ApiError::Validation(format!("inference status failed: {error}")))?;
        if status.active_stem_model.is_none() {
            let loaded = client.load_model("demucs-4stem").map_err(|error| {
                ApiError::Validation(format!("loading demucs-4stem failed: {error}"))
            })?;
            if loaded.active_stem_model.is_none() {
                return Err(ApiError::Validation(
                    "demucs-4stem could not be loaded; download via `alp-cli model download demucs-4stem` or set ALP_DEMUCS_DIR".into(),
                ));
            }
        }
        let stems = client
            .separate_stems(&mixed)
            .map_err(|error| ApiError::Validation(format!("stem separation failed: {error}")))?;
        std::fs::create_dir_all(&output_dir)?;
        let mut files = Vec::with_capacity(4);
        for (name, pcm) in API_DUB_STEM_NAMES.iter().zip(stems.iter()) {
            let path = output_dir.join(format!("{name}.wav"));
            export(&path, pcm, &ExportSettings::wav(WavBitDepth::Int16))?;
            files.push(ApiStemFileDto {
                name: (*name).to_owned(),
                path: path.display().to_string(),
            });
        }
        Ok(DubbingStemsReport {
            stems: files,
            sample_rate: stems[0].sample_rate(),
        })
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

    #[test]
    fn separate_stems_checks_sandbox_and_reports_unavailable_daemon_honestly() {
        let dir = tempdir().expect("tempdir");
        let input = dir.path().join("mix.wav");
        write_test_tone(&input, 0.2, 44_100, 2, 440.0, 0.2).expect("write mix");
        let mut session = session_in(dir.path());
        let escape = session
            .dubbing_separate_stems(&DubbingStemsRequestDto {
                audio_path: "/etc/passwd".into(),
                output_dir: dir.path().join("stems").display().to_string(),
            })
            .expect_err("sandbox blocks outside path");
        assert!(matches!(escape, ApiError::SandboxEscape(_)));

        let result = session.dubbing_separate_stems(&DubbingStemsRequestDto {
            audio_path: input.display().to_string(),
            output_dir: dir.path().join("stems").display().to_string(),
        });
        // Without a running daemon/model this returns a clear Validation error, never a panic.
        assert!(result.is_err());
    }
}
