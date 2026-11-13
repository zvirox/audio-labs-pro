//! Script and voice operations for `ApiSession` (Phase 10 automation).
//!
//! `tts_script_generate` synthesizes every line of a script, imports it as
//! media and lays the clips out on a track. `tts_regenerate` replaces a clip's
//! audio with a fresh synthesis of new text. Both group their commands into a
//! single undo step.

use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::session::ApiSession;
use crate::types::PermissionTier;

/// Default silence inserted between generated lines, in seconds.
const DEFAULT_LINE_GAP_SECS: f64 = 0.35;

/// Result of a script regeneration operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptRegenerateReport {
    /// Clip ID that was regenerated.
    pub clip_id: String,
    /// New clip ID that replaced it.
    pub new_clip_id: String,
    /// New transcript text.
    pub new_text: String,
    /// Engine that ran the synthesis.
    pub engine: String,
    /// Duration of the new clip.
    pub duration_secs: f64,
    /// Timeline start of the replaced clip, in seconds.
    pub start_secs: f64,
    /// Whether the replacement formed a single undo step.
    pub single_undo_step: bool,
}

/// One script line to synthesize.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptLine {
    /// Text to speak.
    pub text: String,
    /// Voice profile handle for this line.
    pub voice_id: String,
    /// Optional speed multiplier.
    #[serde(default)]
    pub speed: Option<f32>,
    /// Optional silence after this line, in seconds.
    #[serde(default)]
    pub gap_secs: Option<f64>,
}

/// A script document: either an object with `segments`/`lines`, or a bare array.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum ScriptDocument {
    Wrapped { segments: Vec<ScriptLine> },
    Bare(Vec<ScriptLine>),
}

impl ScriptDocument {
    fn into_lines(self) -> Vec<ScriptLine> {
        match self {
            ScriptDocument::Wrapped { segments } => segments,
            ScriptDocument::Bare(lines) => lines,
        }
    }
}

/// Result of generating a script.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptGenerateReport {
    /// Track the clips were placed on.
    pub track_id: String,
    /// Clips created, in timeline order.
    pub clip_ids: Vec<String>,
    /// Engines that actually ran, one per line.
    pub engines: Vec<String>,
    /// Total audio duration placed on the timeline, in seconds.
    pub total_duration_secs: f64,
    /// Timeline end of the last clip.
    pub end_secs: f64,
    /// Whether the generated clips form a single undo step.
    pub single_undo_step: bool,
    /// Background job that produced this result (query it with `job_status`).
    pub job_id: String,
}

impl ApiSession {
    /// Regenerate audio for a specific clip: synthesize new text, import it,
    /// and replace the clip in place so its neighbours keep their positions.
    pub fn tts_regenerate(
        &mut self,
        clip_id: &str,
        new_text: &str,
        voice_id: &str,
        speed: Option<f32>,
    ) -> Result<ScriptRegenerateReport, ApiError> {
        self.check_permission(PermissionTier::Edit)?;

        let (track_id, start_secs) = self.clip_start(clip_id)?;
        let undo_before = self.stack.undo_len();

        let generated = self.tts_generate(new_text, voice_id, speed)?;
        let imported =
            self.media_import(&generated.path, Some(&format!("Regenerated {clip_id}")))?;
        self.clip_remove(clip_id)?;
        let placed = self.clip_add(
            &track_id.to_string(),
            &imported.source_id,
            start_secs,
            None,
            None,
            None,
        )?;

        let new_clip_id = placed.clip_id.clone();
        let single_undo_step = self.group_commands_since("Regenerate clip", undo_before);

        Ok(ScriptRegenerateReport {
            clip_id: clip_id.to_owned(),
            new_clip_id,
            new_text: new_text.to_owned(),
            engine: generated.engine,
            duration_secs: generated.duration_secs,
            start_secs,
            single_undo_step,
        })
    }

    /// Generate audio from a script document (one clip per line or paragraph).
    ///
    /// The script is JSON: either `{"segments": [...]}` or a bare array, where
    /// each item is `{ "text", "voice_id", "speed"?, "gap_secs"? }`.
    pub fn tts_script_generate(
        &mut self,
        script_json: &str,
    ) -> Result<ScriptGenerateReport, ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        let job_id = self.jobs.submit("generate script");
        let result = self.tts_script_generate_inner(script_json);
        match &result {
            Ok(report) => self
                .jobs
                .complete(&job_id, format!("{} clip(s)", report.clip_ids.len())),
            Err(error) => self.jobs.fail(&job_id, error.to_string()),
        }
        result.map(|mut report| {
            report.job_id = job_id;
            report
        })
    }

    /// Script generation body; `tts_script_generate` wraps it in a job record.
    fn tts_script_generate_inner(
        &mut self,
        script_json: &str,
    ) -> Result<ScriptGenerateReport, ApiError> {
        let document: ScriptDocument = serde_json::from_str(script_json).map_err(|error| {
            ApiError::Validation(format!(
                "invalid script JSON: {error}; expected {{\"segments\":[{{\"text\":...,\"voice_id\":...}}]}} or an array of those"
            ))
        })?;
        let lines = document.into_lines();
        if lines.is_empty() {
            return Err(ApiError::Validation(
                "script has no lines; add at least one {text, voice_id} item".into(),
            ));
        }
        for (index, line) in lines.iter().enumerate() {
            if line.text.trim().is_empty() {
                return Err(ApiError::Validation(format!(
                    "script line {index} has empty text"
                )));
            }
            if line.voice_id.trim().is_empty() {
                return Err(ApiError::Validation(format!(
                    "script line {index} has no voice_id"
                )));
            }
        }

        let undo_before = self.stack.undo_len();
        let track_id = self.ensure_voice_track()?;
        let track_handle = track_id.to_string();
        let mut cursor = 0.0f64;
        let mut clip_ids = Vec::new();
        let mut engines = Vec::new();
        let mut total_audio = 0.0f64;

        for (index, line) in lines.iter().enumerate() {
            let generated = self.tts_generate(&line.text, &line.voice_id, line.speed)?;
            let imported =
                self.media_import(&generated.path, Some(&format!("Script line {}", index + 1)))?;
            let placed = self.clip_add(
                &track_handle,
                &imported.source_id,
                cursor,
                Some(generated.duration_secs),
                None,
                None,
            )?;
            cursor += generated.duration_secs;
            total_audio += generated.duration_secs;
            clip_ids.push(placed.clip_id);
            engines.push(generated.engine);
            cursor += line.gap_secs.unwrap_or(DEFAULT_LINE_GAP_SECS).max(0.0);
        }

        let single_undo_step = self.group_commands_since("Generate script", undo_before);
        Ok(ScriptGenerateReport {
            track_id: track_handle,
            clip_ids,
            engines,
            total_duration_secs: total_audio,
            end_secs: cursor,
            single_undo_step,
            job_id: String::new(),
        })
    }

    /// Find the clip's owning track and its timeline start.
    fn clip_start(&self, clip_id: &str) -> Result<(alp_core::TrackId, f64), ApiError> {
        let parsed: alp_core::ClipId = clip_id
            .parse()
            .map_err(|_| ApiError::Validation(format!("invalid clip id '{clip_id}'")))?;
        for track in &self.project.tracks {
            if let Some(clip) = track.clips.iter().find(|clip| clip.id == parsed) {
                let start = alp_core::frames_to_secs(clip.timeline_start, self.project.sample_rate);
                return Ok((track.id, start));
            }
        }
        Err(ApiError::NotFound(format!("clip '{clip_id}' not found")))
    }

    /// Reuse a track named "Script" when one exists, otherwise create it.
    fn ensure_voice_track(&mut self) -> Result<alp_core::TrackId, ApiError> {
        if let Some(track) = self.project.tracks.iter().find(|t| t.name == "Script") {
            return Ok(track.id);
        }
        let handle = self.add_track("Script")?;
        handle
            .parse()
            .map_err(|_| ApiError::Validation("new track id was not parseable".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_script_document_accepts_both_shapes() {
        let wrapped: ScriptDocument =
            serde_json::from_str(r#"{"segments":[{"text":"hi","voice_id":"a"}]}"#)
                .expect("wrapped");
        assert_eq!(wrapped.into_lines().len(), 1);

        let bare: ScriptDocument = serde_json::from_str(
            r#"[{"text":"hi","voice_id":"a"},{"text":"there","voice_id":"b"}]"#,
        )
        .expect("bare");
        assert_eq!(bare.into_lines().len(), 2);
    }

    #[test]
    fn a_script_without_voice_ids_is_rejected_before_synthesis() {
        let document: ScriptDocument =
            serde_json::from_str(r#"[{"text":"hi","voice_id":""}]"#).expect("parse");
        assert_eq!(document.into_lines()[0].voice_id, "");
    }
}
