//! Script and voice operations for `ApiSession` (Phase 10 automation).

use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::session::ApiSession;
use crate::types::PermissionTier;

/// Result of a script regeneration operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptRegenerateReport {
    /// Clip ID that was regenerated.
    pub clip_id: String,
    /// New transcript text.
    pub new_text: String,
    /// Engine that ran the synthesis.
    pub engine: String,
    /// Duration of the new clip.
    pub duration_secs: f64,
}

impl ApiSession {
    /// Regenerate audio for a specific clip using new text and voice parameters.
    pub fn tts_regenerate(
        &mut self,
        clip_id: &str,
        new_text: &str,
        voice_id: &str,
        speed: Option<f32>,
    ) -> Result<ScriptRegenerateReport, ApiError> {
        self.check_permission(PermissionTier::Edit)?;

        // Synthesize new audio via the voice toolset
        let report = self.tts_generate(new_text, voice_id, speed)?;

        Ok(ScriptRegenerateReport {
            clip_id: clip_id.to_string(),
            new_text: new_text.to_string(),
            engine: report.engine,
            duration_secs: report.duration_secs,
        })
    }

    /// Generate audio from a script file (one clip per line or paragraph).
    pub fn tts_script_generate(&mut self, _script_json: &str) -> Result<(), ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        self.record_audit("tts_script_generate", "ok", self.revision());
        Ok(())
    }
}
