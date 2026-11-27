//! Desktop UI bridges to real `alp-engine`, `alp-voice`, `alp-dsp`, and `alp-api` capabilities.
//!
//! Keeps `alp-desktop` strictly depending only on `alp-session` while exposing
//! real audio device enumeration, plugin scanning, neural model verification,
//! voice profiles, TTS synthesis, spectral repair, loudness metering, and STT.

use std::path::{Path, PathBuf};

use alp_api::{
    ApiSession, AuditLogger, PermissionTier, SpectralRepairReport, SpectralRepairRequestDto,
    TimeStretchReport, TimeStretchRequestDto, TranscriptionReport, WorkspaceSandbox,
};
use alp_core::{Project, ProjectPaths};
use alp_dsp::{measure_lufs, measure_true_peak_dbfs};
use alp_engine::{DeviceDirection, list_devices};
use alp_media::decode;
use alp_voice::{
    HardwareProfile, MarkupParser, ModelId, ModelKind, ModelManager, ModelStatus, ProfileStore,
    PronunciationDictionary, SpeechToken, VoiceKind,
};
use serde::{Deserialize, Serialize};

/// Serializable audio device entry matching frontend `AudioDevice`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioDeviceDto {
    pub name: String,
    pub is_default: bool,
    pub direction: String,
    pub channels: u16,
    pub sample_rate: u32,
    pub sample_format: String,
}

/// Serializable catalogue entry matching frontend `ModelItem`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelCatalogueItemDto {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub size_mb: f64,
    pub status: String,
    pub license: String,
    pub spdx: String,
    pub description: String,
    pub is_builtin: bool,
    pub license_accepted: bool,
}

/// Host compute telemetry from `alp-voice::HardwareProfile::probe()`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HardwareProfileDto {
    pub total_ram_gb: f64,
    pub available_ram_gb: f64,
    pub cpu_cores: usize,
    pub gpu_detected: bool,
    pub gpu_description: Option<String>,
    pub recommended_tier: String,
}

/// Discovered native CLAP / LV2 / VST3 plugin descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredPluginDto {
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub format: String,
    pub category: String,
    pub path: String,
}

/// Consent record DTO matching frontend `ConsentRecord`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesktopConsentDto {
    pub speaker_name: String,
    pub attested_by: String,
    pub timestamp: u64,
    pub statement: String,
}

/// Voice profile DTO matching frontend `VoiceProfile`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesktopVoiceProfileDto {
    pub id: String,
    pub name: String,
    pub language: String,
    pub tags: Vec<String>,
    pub kind: String,
    pub speed: f32,
    pub pitch: f32,
    pub energy: f32,
    pub emotion: String,
    pub consent: Option<DesktopConsentDto>,
}

/// Result of desktop speech synthesis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesktopTtsReportDto {
    pub path: String,
    pub duration_secs: f64,
    pub sample_rate: u32,
    pub engine: String,
    pub voice_id: String,
}

/// One pronunciation override rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LexiconRuleDto {
    pub word: String,
    pub replacement: String,
}

/// Parsed and substituted lexicon preview result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LexiconPreviewDto {
    pub output_text: String,
    pub tokens: Vec<String>,
}

/// Decoded audio file loudness telemetry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileLoudnessDto {
    pub path: String,
    pub integrated_lufs: f32,
    pub peak_dbfs: f32,
    pub true_peak_dbfs: f32,
    pub duration_secs: f64,
    pub sample_rate: u32,
    pub channels: u16,
}

fn temp_desktop_api_session(roots: Vec<PathBuf>) -> Result<ApiSession, String> {
    let base = std::env::temp_dir().join("alp-desktop-ops");
    std::fs::create_dir_all(&base).map_err(|e| format!("failed to prepare work dir: {e}"))?;
    let mut sandbox_roots = vec![base.clone()];
    for root in roots {
        if root.exists() && !sandbox_roots.contains(&root) {
            sandbox_roots.push(root);
        }
    }
    let sandbox = WorkspaceSandbox::new(sandbox_roots).map_err(|e| e.to_string())?;
    let audit = AuditLogger::new(base.join("audit.jsonl")).map_err(|e| e.to_string())?;
    let project = Project::new("Desktop Bridge").map_err(|e| e.to_string())?;
    let paths = ProjectPaths::new(base.join("bridge.alp"));
    ApiSession::new(
        project,
        paths,
        PermissionTier::Export,
        sandbox,
        audit,
        "desktop-ui",
    )
    .map_err(|e| e.to_string())
}

fn parent_roots(paths: &[&str]) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for raw in paths {
        let p = Path::new(raw);
        if let Some(parent) = p.parent()
            && !parent.as_os_str().is_empty()
        {
            roots.push(parent.to_path_buf());
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd);
    }
    roots
}

/// Enumerate real system ALSA / PipeWire audio output and input devices.
#[must_use]
pub fn list_audio_devices() -> Vec<AudioDeviceDto> {
    match list_devices() {
        Ok(devs) => devs
            .into_iter()
            .map(|d| AudioDeviceDto {
                name: d.name,
                is_default: d.is_default,
                direction: match d.direction {
                    DeviceDirection::Output => "output".into(),
                    DeviceDirection::Input => "input".into(),
                },
                channels: d.channels,
                sample_rate: d.sample_rate,
                sample_format: d.sample_format.to_ascii_lowercase(),
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn model_item_dto(
    manager: &ModelManager,
    manifest: &alp_voice::ModelManifest,
) -> ModelCatalogueItemDto {
    let kind = match manifest.kind {
        ModelKind::TtsLight => "tts_light",
        ModelKind::TtsMedium => "tts_medium",
        ModelKind::Stt => "stt",
        ModelKind::VoiceClone => "cloning",
        ModelKind::StemSeparation => "stem_separation",
    }
    .to_string();
    let status = match manager.status(&manifest.id) {
        ModelStatus::NotInstalled => "not_installed",
        ModelStatus::Downloading { .. } => "downloading",
        ModelStatus::Installed { .. } => "installed",
        ModelStatus::Verified { .. } => "verified",
        ModelStatus::Corrupt { .. } => "corrupt",
    }
    .to_string();
    let size_mb = ((manifest.total_size_bytes() as f64 / (1024.0 * 1024.0)) * 10.0).round() / 10.0;
    ModelCatalogueItemDto {
        id: manifest.id.as_str().to_string(),
        name: manifest.name.clone(),
        kind,
        size_mb,
        status,
        license: manifest.license.name.clone(),
        spdx: manifest.license.spdx.clone(),
        description: manifest.description.clone(),
        is_builtin: manifest.is_builtin,
        license_accepted: manifest.license.accepted,
    }
}

/// Return real local neural model catalogue and installation state.
#[must_use]
pub fn list_model_catalogue() -> Vec<ModelCatalogueItemDto> {
    let manager = ModelManager::new();
    manager
        .catalogue()
        .iter()
        .map(|m| model_item_dto(&manager, m))
        .collect()
}

/// Verify SHA-256 integrity of an installed model on disk.
pub fn verify_model_integrity(model_id: &str) -> Result<ModelCatalogueItemDto, String> {
    let manager = ModelManager::new();
    let id = ModelId::new(model_id);
    let status = manager.verify(&id).map_err(|e| e.to_string())?;
    let manifest = manager
        .get_manifest(&id)
        .ok_or_else(|| format!("unknown model '{model_id}'"))?;
    let mut dto = model_item_dto(&manager, manifest);
    dto.status = match status {
        ModelStatus::Verified { .. } => "verified".into(),
        ModelStatus::Installed { .. } => "installed".into(),
        ModelStatus::Corrupt { .. } => "corrupt".into(),
        ModelStatus::Downloading { .. } => "downloading".into(),
        ModelStatus::NotInstalled => "not_installed".into(),
    };
    Ok(dto)
}

/// Probe host CPU cores, RAM, and GPU acceleration availability.
#[must_use]
pub fn probe_hardware_profile() -> HardwareProfileDto {
    let hw = HardwareProfile::probe();
    HardwareProfileDto {
        total_ram_gb: (hw.total_ram_gb() * 10.0).round() / 10.0,
        available_ram_gb: (hw.available_ram_gb() * 10.0).round() / 10.0,
        cpu_cores: hw.cpu_cores,
        gpu_detected: hw.gpu_detected,
        gpu_description: hw.gpu_description,
        recommended_tier: hw.recommended_tier.as_str().to_string(),
    }
}

/// Discover real CLAP, LV2, and VST3 plugins installed on Linux.
pub fn discover_system_plugins() -> Result<Vec<DiscoveredPluginDto>, String> {
    let session = temp_desktop_api_session(Vec::new())?;
    let descriptors = session.plugin_list().map_err(|e| e.to_string())?;
    Ok(descriptors
        .into_iter()
        .map(|p| DiscoveredPluginDto {
            id: p.id,
            name: p.name,
            vendor: p.vendor,
            format: p.format.name().to_string(),
            category: format!("{:?}", p.category),
            path: p.path.display().to_string(),
        })
        .collect())
}

/// List voice profiles from the persistent local profile store.
pub fn list_voice_profiles() -> Result<Vec<DesktopVoiceProfileDto>, String> {
    let store = ProfileStore::default_store().map_err(|e| e.to_string())?;
    Ok(store
        .list_profiles()
        .iter()
        .map(|p| DesktopVoiceProfileDto {
            id: p.id.clone(),
            name: p.name.clone(),
            language: "English".into(),
            tags: p.tags.clone(),
            kind: match p.kind {
                VoiceKind::Builtin => "acoustic".into(),
                VoiceKind::Designed => "prebuilt".into(),
                VoiceKind::Cloned => "cloned".into(),
            },
            speed: p.default_params.speed,
            pitch: p.default_params.pitch,
            energy: p.default_params.energy,
            emotion: p.default_params.emotion.clone(),
            consent: p.consent.as_ref().map(|c| DesktopConsentDto {
                speaker_name: c.speaker_name.clone(),
                attested_by: c.attested_by.clone(),
                timestamp: c.timestamp,
                statement: c.statement.clone(),
            }),
        })
        .collect())
}

/// Synthesize a speech take via `ApiSession` (`alp-infer` daemon first, built-in fallback).
pub fn synthesize_tts_preview(
    voice_id: &str,
    text: &str,
    speed: f32,
) -> Result<DesktopTtsReportDto, String> {
    let mut session = temp_desktop_api_session(Vec::new())?;
    let report = session
        .tts_generate(text, voice_id, Some(speed))
        .map_err(|e| e.to_string())?;
    Ok(DesktopTtsReportDto {
        path: report.path,
        duration_secs: report.duration_secs,
        sample_rate: report.sample_rate,
        engine: report.engine,
        voice_id: voice_id.to_string(),
    })
}

/// Apply pronunciation dictionary replacements and parse inline `[pause=...]` / `[emphasis]` markup.
#[must_use]
pub fn preview_lexicon_markup(rules: Vec<LexiconRuleDto>, text: &str) -> LexiconPreviewDto {
    let mut dict = PronunciationDictionary::new();
    for rule in rules {
        if !rule.word.trim().is_empty() && !rule.replacement.trim().is_empty() {
            dict.add(rule.word.trim(), rule.replacement.trim());
        }
    }
    let output_text = dict.apply(text);
    let tokens = MarkupParser::parse(&output_text)
        .into_iter()
        .map(|tok| match tok {
            SpeechToken::Words(w) => format!("TEXT: {w}"),
            SpeechToken::PauseMs(ms) => format!("PAUSE: {ms} ms"),
            SpeechToken::Emphasis(w) => format!("EMPHASIS: {w}"),
        })
        .collect();
    LexiconPreviewDto {
        output_text,
        tokens,
    }
}

/// Run offline notch or harmonic spectral repair on an audio file.
pub fn desktop_spectral_repair_file(
    request: &SpectralRepairRequestDto,
) -> Result<SpectralRepairReport, String> {
    let mut session =
        temp_desktop_api_session(parent_roots(&[&request.input_path, &request.output_path]))?;
    session
        .dsp_spectral_repair(request)
        .map_err(|e| e.to_string())
}

/// Run offline WSOLA time-stretching on an audio file.
pub fn desktop_time_stretch_file(
    request: &TimeStretchRequestDto,
) -> Result<TimeStretchReport, String> {
    let mut session =
        temp_desktop_api_session(parent_roots(&[&request.input_path, &request.output_path]))?;
    session.dsp_time_stretch(request).map_err(|e| e.to_string())
}

/// Measure integrated LUFS, sample peak, and true peak of a local audio file.
pub fn analyze_file_loudness(path: &str) -> Result<FileLoudnessDto, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("audio path cannot be empty".into());
    }
    let pcm = decode(Path::new(trimmed)).map_err(|e| e.to_string())?;
    let integrated_lufs = measure_lufs(pcm.samples(), pcm.sample_rate(), pcm.channels());
    let sample_peak = pcm.peak().max(1e-6);
    let peak_dbfs = 20.0 * sample_peak.log10();
    let true_peak_dbfs = measure_true_peak_dbfs(pcm.samples(), pcm.sample_rate(), pcm.channels());
    Ok(FileLoudnessDto {
        path: trimmed.to_string(),
        integrated_lufs,
        peak_dbfs,
        true_peak_dbfs,
        duration_secs: pcm.duration_secs(),
        sample_rate: pcm.sample_rate(),
        channels: pcm.channels(),
    })
}

/// Transcribe a local audio file via `ApiSession` (`alp-infer` Moonshine first, built-in fallback).
pub fn desktop_transcribe_audio(path: &str) -> Result<TranscriptionReport, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("audio path cannot be empty".into());
    }
    let mut session = temp_desktop_api_session(parent_roots(&[trimmed]))?;
    session
        .transcribe(trimmed, None, None)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexicon_applies_rules_and_markup() {
        let out = preview_lexicon_markup(
            vec![LexiconRuleDto {
                word: "ALSA".into(),
                replacement: "ahl-sah".into(),
            }],
            "Testing ALSA [pause=300ms] now.",
        );
        assert!(out.output_text.contains("ahl-sah"));
        assert!(out.tokens.iter().any(|t| t == "PAUSE: 300 ms"));
    }

    #[test]
    fn hardware_and_models_return_real_catalogue() {
        let hw = probe_hardware_profile();
        assert!(hw.cpu_cores >= 1);
        let models = list_model_catalogue();
        assert!(models.iter().any(|m| m.id == "demucs-4stem"));
    }

    #[test]
    fn empty_loudness_path_returns_error() {
        assert!(analyze_file_loudness("  ").is_err());
    }
}
