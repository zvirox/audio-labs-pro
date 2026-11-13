//! Effect catalog, application, and preset operations for `ApiSession`.

use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::session::ApiSession;
use crate::types::PermissionTier;
use alp_core::{EffectRef, SetTrackEffects};

/// Built-in DSP effect descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectDescriptor {
    /// Effect identifier (matches `alp-dsp` effect names).
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Category for UI grouping.
    pub category: String,
    /// Parameter schema as JSON Schema.
    pub params_schema: serde_json::Value,
}

/// Built-in preset descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetDescriptor {
    /// Preset identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Description of what the preset does.
    pub description: String,
}

/// Catalog of available effects and presets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectCatalogReport {
    /// Individual effects available for insertion.
    pub effects: Vec<EffectDescriptor>,
    /// Pre-configured effect chains (presets).
    pub presets: Vec<PresetDescriptor>,
}

impl ApiSession {
    /// List all built-in effects and presets available in the catalog.
    pub fn effect_catalog(&self) -> Result<EffectCatalogReport, ApiError> {
        self.check_permission(PermissionTier::Observe)?;
        Ok(EffectCatalogReport {
            effects: vec![
                EffectDescriptor {
                    id: "parametric_eq".into(),
                    name: "Parametric EQ".into(),
                    category: "eq".into(),
                    params_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "bands": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "filter_type": { "type": "string", "enum": ["highpass", "lowpass", "peaking", "highshelf", "lowshelf"] },
                                        "frequency_hz": { "type": "number" },
                                        "q": { "type": "number" },
                                        "gain_db": { "type": "number" }
                                    },
                                    "required": ["filter_type", "frequency_hz", "q", "gain_db"]
                                }
                            }
                        },
                        "required": ["bands"]
                    }),
                },
                EffectDescriptor {
                    id: "compressor".into(),
                    name: "Compressor".into(),
                    category: "dynamics".into(),
                    params_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "threshold_db": { "type": "number" },
                            "ratio": { "type": "number" },
                            "attack_ms": { "type": "number" },
                            "release_ms": { "type": "number" },
                            "makeup_db": { "type": "number" }
                        },
                        "required": ["threshold_db", "ratio"]
                    }),
                },
                EffectDescriptor {
                    id: "limiter".into(),
                    name: "True Peak Limiter".into(),
                    category: "dynamics".into(),
                    params_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "ceiling_db": { "type": "number" },
                            "release_ms": { "type": "number" }
                        },
                        "required": ["ceiling_db"]
                    }),
                },
                EffectDescriptor {
                    id: "de_esser".into(),
                    name: "De-Esser".into(),
                    category: "voice".into(),
                    params_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "frequency_hz": { "type": "number" },
                            "threshold_db": { "type": "number" },
                            "ratio": { "type": "number" }
                        },
                        "required": ["frequency_hz", "threshold_db", "ratio"]
                    }),
                },
                EffectDescriptor {
                    id: "de_hum".into(),
                    name: "De-Hum".into(),
                    category: "voice".into(),
                    params_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "mains_hz": { "type": "number" },
                            "q": { "type": "number" }
                        },
                        "required": ["mains_hz"]
                    }),
                },
                EffectDescriptor {
                    id: "de_click".into(),
                    name: "De-Click".into(),
                    category: "voice".into(),
                    params_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "sensitivity": { "type": "number" }
                        },
                        "required": ["sensitivity"]
                    }),
                },
                EffectDescriptor {
                    id: "noise_gate".into(),
                    name: "Noise Gate".into(),
                    category: "dynamics".into(),
                    params_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "threshold_db": { "type": "number" },
                            "ratio": { "type": "number" },
                            "attack_ms": { "type": "number" },
                            "release_ms": { "type": "number" }
                        },
                        "required": ["threshold_db", "ratio"]
                    }),
                },
            ],
            presets: vec![
                PresetDescriptor {
                    id: "voice_cleanup".into(),
                    name: "Voice Cleanup".into(),
                    description: "One-click vocal chain: HPF 80 Hz → De-Hum → De-Click → Noise Gate → Parametric EQ → De-Esser → Vocal Compressor → True Peak Limiter".into(),
                },
                PresetDescriptor {
                    id: "podcast_master".into(),
                    name: "Podcast Master".into(),
                    description: "Full podcast mastering chain: Voice Cleanup → Loudness Normalize to -16 LUFS → True Peak Limit to -1 dBFS".into(),
                },
            ],
        })
    }

    /// Apply an effect to a track's effect chain.
    pub fn effect_apply(
        &mut self,
        track_id: &str,
        effect_id: &str,
        params: serde_json::Value,
    ) -> Result<String, ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        self.validate_effect_id(effect_id)?;
        let track = self
            .project
            .tracks
            .iter()
            .find(|trk| trk.id.to_string() == track_id)
            .ok_or_else(|| ApiError::NotFound(format!("track '{track_id}' not found")))?;
        let track_actual_id = track.id;
        let mut effects = track.effects.clone();

        let new_effect_id = format!("fx-{}", uuid::Uuid::new_v4().simple());
        let mut effect = EffectRef::new(&new_effect_id, effect_id);
        if let Some(map) = params.as_object() {
            for (k, v) in map {
                if let Some(num) = v.as_f64() {
                    effect.params.insert(k.clone(), num);
                }
            }
        }
        effects.push(effect);

        self.execute_command(
            "effect_apply",
            Box::new(SetTrackEffects::new(track_actual_id, effects)),
        )?;
        Ok(new_effect_id)
    }

    /// Update parameters of an already-inserted effect.
    pub fn effect_update(
        &mut self,
        track_id: &str,
        effect_id: &str,
        params: serde_json::Value,
    ) -> Result<(), ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        let track = self
            .project
            .tracks
            .iter()
            .find(|trk| trk.id.to_string() == track_id)
            .ok_or_else(|| ApiError::NotFound(format!("track '{track_id}' not found")))?;
        let track_actual_id = track.id;
        let mut effects = track.effects.clone();

        let fx = effects
            .iter_mut()
            .find(|e| e.id == effect_id)
            .ok_or_else(|| {
                ApiError::NotFound(format!("effect '{effect_id}' not found on track"))
            })?;
        if let Some(map) = params.as_object() {
            for (k, v) in map {
                if let Some(num) = v.as_f64() {
                    fx.params.insert(k.clone(), num);
                }
            }
        }

        self.execute_command(
            "effect_update",
            Box::new(SetTrackEffects::new(track_actual_id, effects)),
        )
    }

    /// Remove an effect from a track's effect chain.
    pub fn effect_remove(&mut self, track_id: &str, effect_id: &str) -> Result<(), ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        let track = self
            .project
            .tracks
            .iter()
            .find(|trk| trk.id.to_string() == track_id)
            .ok_or_else(|| ApiError::NotFound(format!("track '{track_id}' not found")))?;
        let track_actual_id = track.id;
        let mut effects = track.effects.clone();

        let initial_len = effects.len();
        effects.retain(|e| e.id != effect_id);
        if effects.len() == initial_len {
            return Err(ApiError::NotFound(format!(
                "effect '{effect_id}' not found on track"
            )));
        }

        self.execute_command(
            "effect_remove",
            Box::new(SetTrackEffects::new(track_actual_id, effects)),
        )
    }

    /// Reject an effect id the catalog does not offer, so a typo fails loudly
    /// instead of inserting an inert chain entry that never does anything.
    fn validate_effect_id(&self, effect_id: &str) -> Result<(), ApiError> {
        let catalog = self.effect_catalog()?;
        if catalog.effects.iter().any(|effect| effect.id == effect_id) {
            return Ok(());
        }
        let known: Vec<&str> = catalog
            .effects
            .iter()
            .map(|effect| effect.id.as_str())
            .collect();
        Err(ApiError::Validation(format!(
            "unknown effect '{effect_id}'; call effect_catalog for the list ({})",
            known.join(", ")
        )))
    }

    /// Reject a preset the catalog does not offer.
    fn validate_preset(&self, preset: &str) -> Result<(), ApiError> {
        let catalog = self.effect_catalog()?;
        if catalog.presets.iter().any(|entry| entry.id == preset) {
            return Ok(());
        }
        let known: Vec<&str> = catalog
            .presets
            .iter()
            .map(|entry| entry.id.as_str())
            .collect();
        Err(ApiError::Validation(format!(
            "unknown preset '{preset}'; call effect_catalog for the list ({})",
            known.join(", ")
        )))
    }

    /// Apply a named preset chain to a track (e.g., "voice_cleanup").
    pub fn preset_apply(&mut self, track_id: &str, preset: &str) -> Result<(), ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        self.validate_preset(preset)?;
        let track = self
            .project
            .tracks
            .iter()
            .find(|trk| trk.id.to_string() == track_id)
            .ok_or_else(|| ApiError::NotFound(format!("track '{track_id}' not found")))?;
        let track_actual_id = track.id;

        let effect_id = format!("fx-{}", uuid::Uuid::new_v4().simple());
        let effect = EffectRef::new(&effect_id, format!("preset:{preset}"));
        let effects = vec![effect];

        self.execute_command(
            "preset_apply",
            Box::new(SetTrackEffects::new(track_actual_id, effects)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::AuditLogger;
    use crate::sandbox::WorkspaceSandbox;
    use alp_core::{Project, ProjectPaths, Track, TrackId};
    use tempfile::tempdir;

    fn session() -> (ApiSession, String) {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().to_path_buf();
        std::mem::forget(dir);
        let paths = ProjectPaths::new(&path);
        paths.ensure_layout().expect("layout");
        let mut project = Project::new("Effects").expect("project");
        let track = Track::new(TrackId::new(), "Voice");
        let track_id = track.id.to_string();
        project.tracks.push(track);
        let sandbox = WorkspaceSandbox::new([&path]).expect("sandbox");
        let audit = AuditLogger::new(path.join("audit.jsonl")).expect("audit");
        let session = ApiSession::new(
            project,
            paths,
            PermissionTier::Edit,
            sandbox,
            audit,
            "agent",
        )
        .expect("session");
        (session, track_id)
    }

    #[test]
    fn a_known_effect_lands_in_the_chain() {
        let (mut session, track_id) = session();
        let handle = session
            .effect_apply(
                &track_id,
                "compressor",
                serde_json::json!({ "threshold_db": -18.0 }),
            )
            .expect("apply compressor");
        let track = &session.project.tracks[0];
        assert_eq!(track.effects.len(), 1);
        assert_eq!(track.effects[0].id, handle);
        assert_eq!(track.effects[0].kind, "compressor");
        assert_eq!(track.effects[0].params.get("threshold_db"), Some(&-18.0));
    }

    #[test]
    fn an_unknown_effect_id_is_rejected_with_the_catalog() {
        let (mut session, track_id) = session();
        let error = session
            .effect_apply(&track_id, "not_an_effect", serde_json::json!({}))
            .expect_err("unknown effect must fail");
        assert!(error.to_string().contains("effect_catalog"), "{error}");
        assert!(session.project.tracks[0].effects.is_empty());
    }

    #[test]
    fn an_unknown_preset_is_rejected_and_a_known_one_applies() {
        let (mut session, track_id) = session();
        let error = session
            .preset_apply(&track_id, "nope")
            .expect_err("unknown preset must fail");
        assert!(error.to_string().contains("voice_cleanup"), "{error}");

        session
            .preset_apply(&track_id, "voice_cleanup")
            .expect("known preset applies");
        assert_eq!(session.project.tracks[0].effects.len(), 1);
    }
}
