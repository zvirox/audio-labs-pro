//! Voice design system for generating customized voice profiles from descriptive text.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::VoiceKind;
use crate::profile::{VoiceParams, VoiceProfile};

/// Request payload to synthesize a new voice profile from a natural language brief.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceDesignRequest {
    /// Friendly name for the resulting voice.
    pub name: String,
    /// Descriptive text (e.g. "Warm, deep BBC documentary narrator with calm pacing").
    pub prompt: String,
    /// Categorical tags to attach.
    pub tags: Vec<String>,
}

/// Analyze a voice design prompt and synthesize a customized `VoiceProfile`.
#[must_use]
pub fn design_voice(request: &VoiceDesignRequest) -> VoiceProfile {
    let lower = request.prompt.to_ascii_lowercase();

    let mut speed = 1.0f32;
    let mut pitch = 0.0f32;
    let mut energy = 1.0f32;
    let mut emotion = "neutral".to_string();

    if lower.contains("deep") || lower.contains("low") || lower.contains("resonant") {
        pitch -= 3.5;
    } else if lower.contains("bright") || lower.contains("high") || lower.contains("child") {
        pitch += 4.0;
    }

    if lower.contains("fast") || lower.contains("quick") || lower.contains("urgent") {
        speed = 1.25;
    } else if lower.contains("slow") || lower.contains("calm") || lower.contains("deliberate") {
        speed = 0.85;
    }

    if lower.contains("whisper") || lower.contains("soft") || lower.contains("quiet") {
        energy = 0.65;
        emotion = "whisper".into();
    } else if lower.contains("excited") || lower.contains("shout") || lower.contains("energetic") {
        energy = 1.4;
        emotion = "excited".into();
    } else if lower.contains("warm") || lower.contains("friendly") {
        emotion = "warm".into();
    }

    let default_params = VoiceParams {
        speed,
        pitch,
        energy,
        emotion,
    };

    let id = format!("designed-{}", Uuid::new_v4());
    let mut tags = request.tags.clone();
    if !tags.iter().any(|t| t == "designed") {
        tags.push("designed".into());
    }

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    VoiceProfile {
        id,
        name: request.name.clone(),
        kind: VoiceKind::Designed,
        backend: "builtin-acoustic".into(),
        reference_audio: None,
        transcript: None,
        cached_embedding: None,
        tags,
        default_params,
        created_at: timestamp,
        consent: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn designs_voice_profile_from_prompt_keywords() {
        let req = VoiceDesignRequest {
            name: "Deep Narrator".into(),
            prompt: "A deep, calm, warm documentary voice".into(),
            tags: vec!["narration".into()],
        };

        let profile = design_voice(&req);
        assert_eq!(profile.name, "Deep Narrator");
        assert_eq!(profile.kind, VoiceKind::Designed);
        assert!(profile.default_params.pitch < 0.0); // deep
        assert!(profile.default_params.speed < 1.0); // calm
        assert_eq!(profile.default_params.emotion, "warm");
        assert!(profile.tags.contains(&"designed".to_string()));
    }
}
