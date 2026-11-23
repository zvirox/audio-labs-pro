//! Audiobook encode for `ApiSession` (target-A parity: `audiobook_encode`).
//!
//! An audiobook script is JSON: either `{"segments": [...]}` or a bare array
//! of segments, where each segment is `{ "text", "voice_id", "speed"?,
//! "gap_secs"? }` — the same contract as `tts_script_generate` (see
//! `script_voice_ops`). The encode synthesizes every segment with its voice,
//! places one clip per segment on a dedicated `Audiobook` track, and exports
//! the master with the audiobook delivery preset (`ExportPreset::Audiobook`).
//!
//! A cancelled encode stops between segments: the job stays `Cancelled` and
//! the already-placed clips remain on the timeline.

use serde::{Deserialize, Serialize};

use alp_core::now_ms;
use alp_media::ExportPreset;

use crate::error::ApiError;
use crate::session::ApiSession;
use crate::types::PermissionTier;

/// Default pause inserted between audiobook segments, in seconds (matches the
/// `gap_secs` default used by the script tool).
const DEFAULT_SEGMENT_GAP_SECS: f64 = 0.35;

/// One segment of an audiobook script.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudiobookSegment {
    /// Text to speak for this segment.
    pub text: String,
    /// Voice profile handle for this segment.
    pub voice_id: String,
    /// Optional speed multiplier.
    #[serde(default)]
    pub speed: Option<f32>,
    /// Pause after this segment (except the last), in seconds.
    #[serde(default = "default_segment_gap_secs")]
    pub gap_secs: f64,
}

fn default_segment_gap_secs() -> f64 {
    DEFAULT_SEGMENT_GAP_SECS
}

/// An audiobook script: `{"segments": [...]}` or a bare array of segments.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AudiobookScript {
    /// Object form with a `segments` array.
    Wrapped { segments: Vec<AudiobookSegment> },
    /// Bare array form.
    Bare(Vec<AudiobookSegment>),
}

/// Result of the `audiobook_encode` tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudiobookEncodeReport {
    /// Track the audiobook clips were placed on.
    pub track_id: String,
    /// Path of the exported master audio file.
    pub master_path: String,
    /// Number of segments placed.
    pub segment_count: usize,
    /// Timeline length of the placed audiobook, in seconds.
    pub total_duration_secs: f64,
    /// Background job that produced this result (query it with `job_status`).
    pub job_id: String,
}

impl ApiSession {
    /// Encode a full audiobook (target-A parity: `audiobook render`):
    /// synthesize every script segment with its voice and place the clips on
    /// a dedicated `Audiobook` track, then export the master with the
    /// audiobook delivery preset.
    pub fn audiobook_encode(
        &mut self,
        script_json: &str,
        output_path: Option<&str>,
    ) -> Result<AudiobookEncodeReport, ApiError> {
        self.check_permission(PermissionTier::Export)?;
        let job_id = self.jobs.submit("audiobook encode");
        let result = self.audiobook_encode_inner(script_json, output_path, &job_id);
        match &result {
            Ok(report) => {
                self.jobs
                    .complete(&job_id, format!("{} segment(s)", report.segment_count));
            }
            Err(error) => {
                // Leave an explicit cancellation in place; other failures mark
                // the job failed.
                if !self.jobs.is_cancelled(&job_id) {
                    self.jobs.fail(&job_id, error.to_string());
                }
            }
        }
        result.map(|mut report| {
            report.job_id = job_id;
            report
        })
    }

    fn audiobook_encode_inner(
        &mut self,
        script_json: &str,
        output_path: Option<&str>,
        job_id: &str,
    ) -> Result<AudiobookEncodeReport, ApiError> {
        let script: AudiobookScript = serde_json::from_str(script_json).map_err(|error| {
            ApiError::Validation(format!(
                "invalid audiobook script JSON: {error}; expected {{\"segments\":[{{\"text\":...,\"voice_id\":...}}]}} or an array of those"
            ))
        })?;
        let segments: Vec<AudiobookSegment> = match script {
            AudiobookScript::Wrapped { segments } | AudiobookScript::Bare(segments) => segments,
        };
        if segments.is_empty() {
            return Err(ApiError::Validation(
                "audiobook script has no segments; add at least one {text, voice_id} item".into(),
            ));
        }
        for (index, segment) in segments.iter().enumerate() {
            if segment.text.trim().is_empty() {
                return Err(ApiError::Validation(format!(
                    "segment {index} has empty text"
                )));
            }
            if segment.gap_secs < 0.0 {
                return Err(ApiError::Validation(format!(
                    "segment {index} has a negative gap_secs; use zero or a positive pause"
                )));
            }
        }
        // Fail before placing anything when a voice is unknown.
        let store = self.voice_store()?;
        for segment in &segments {
            if store.get_profile(&segment.voice_id).is_none() {
                return Err(ApiError::NotFound(format!(
                    "voice '{}' not found; call tts_voices for the known ids",
                    segment.voice_id
                )));
            }
        }

        let track_id = self.add_track("Audiobook")?;
        let total = segments.len();
        let mut cursor = 0.0_f64;
        let mut placed = 0_usize;
        for (index, segment) in segments.iter().enumerate() {
            if self.jobs.is_cancelled(job_id) {
                return Err(ApiError::Validation(
                    "audiobook encode was cancelled before completing".into(),
                ));
            }
            let generated =
                self.tts_generate_inner(&segment.text, &segment.voice_id, segment.speed)?;
            let imported = self.media_import(
                &generated.path,
                Some(&format!("audiobook segment {}", index + 1)),
            )?;
            let placement =
                self.clip_add(&track_id, &imported.source_id, cursor, None, None, None)?;
            cursor += placement.duration_secs;
            placed += 1;
            if index + 1 < total {
                cursor += segment.gap_secs;
            }
            self.jobs.update_progress(
                job_id,
                placed as f32 / total as f32,
                format!("Synthesized segment {placed}/{total}"),
            );
        }

        let master_path: String = match output_path.map(str::trim).filter(|path| !path.is_empty()) {
            Some(path) => path.to_owned(),
            None => {
                let name = format!("audiobook-{}.wav", now_ms());
                self.paths.renders_dir().join(&name).display().to_string()
            }
        };
        let master_path = self.export_audio(ExportPreset::Audiobook, master_path)?;
        self.record_audit("audiobook_encode", "ok", self.revision());
        Ok(AudiobookEncodeReport {
            track_id,
            master_path,
            segment_count: placed,
            total_duration_secs: cursor,
            job_id: String::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::AuditLogger;
    use crate::sandbox::WorkspaceSandbox;
    use alp_core::{Project, ProjectPaths};
    use tempfile::tempdir;

    fn session_in(dir: &std::path::Path, tier: PermissionTier) -> ApiSession {
        let proj_paths = ProjectPaths::new(dir.join("proj.alp"));
        proj_paths.ensure_layout().expect("layout");
        let project = Project::new("Audiobook Test").expect("project");
        let sandbox = WorkspaceSandbox::new([dir]).expect("sandbox");
        let audit = AuditLogger::new(dir.join("audit.jsonl")).expect("audit");
        ApiSession::new(project, proj_paths, tier, sandbox, audit, "agent")
            .expect("session")
            .with_voice_store_dir(dir.join("voices"))
    }

    /// Build a `{"segments":[...]}` script from `(voice, text)` pairs.
    fn script(segments: &[(&str, &str)]) -> String {
        let items: Vec<String> = segments
            .iter()
            .map(|(voice, text)| {
                format!(
                    "{{\"text\":{},\"voice_id\":{}}}",
                    serde_json::json!(text),
                    serde_json::json!(voice)
                )
            })
            .collect();
        format!("{{\"segments\":[{}]}}", items.join(","))
    }

    #[test]
    fn audiobook_encode_places_clips_and_exports_master() {
        let dir = tempdir().expect("tempdir");
        let mut session = session_in(dir.path(), PermissionTier::Export);
        let first = session
            .voice_design("Narrator", "calm narrator", vec![])
            .expect("design")
            .id;
        let second = session
            .voice_design("Narrator Two", "warm narrator", vec![])
            .expect("design")
            .id;

        let report = session
            .audiobook_encode(
                &script(&[
                    (first.as_str(), "First segment."),
                    (second.as_str(), "Second segment."),
                ]),
                None,
            )
            .expect("encode");

        assert_eq!(report.segment_count, 2);
        assert!(
            std::path::Path::new(&report.master_path).is_file(),
            "the master export exists"
        );
        assert!(report.total_duration_secs > 0.0, "duration is measured");

        let info = session.project_info().expect("project info");
        assert!(
            info.clip_count >= 2,
            "one clip per segment is placed, got {}",
            info.clip_count
        );

        let status = session.job_status(&report.job_id).expect("job status");
        assert_eq!(status.state, "completed", "the job is queryable");

        // The exported master is real audio (non-empty WAV bytes).
        let master = std::fs::read(&report.master_path).expect("read master");
        assert!(master.len() > 44, "the master has a WAV header and samples");
    }

    #[test]
    fn audiobook_encode_rejects_an_unknown_voice() {
        let dir = tempdir().expect("tempdir");
        let mut session = session_in(dir.path(), PermissionTier::Export);
        let error = session
            .audiobook_encode(&script(&[("missing-voice", "hello")]), None)
            .expect_err("unknown voice");
        assert!(matches!(error, ApiError::NotFound(_)));
    }

    #[test]
    fn audiobook_encode_rejects_an_empty_script() {
        let dir = tempdir().expect("tempdir");
        let mut session = session_in(dir.path(), PermissionTier::Export);
        let error = session
            .audiobook_encode("{\"segments\":[]}", None)
            .expect_err("empty wrapped script");
        assert!(matches!(error, ApiError::Validation(_)));
        let error = session
            .audiobook_encode("[]", None)
            .expect_err("empty bare script");
        assert!(matches!(error, ApiError::Validation(_)));
    }
}
