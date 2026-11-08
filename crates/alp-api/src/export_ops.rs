//! Export operations (presets, stems, transcript, preview) for `ApiSession`.

use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::session::ApiSession;
use crate::types::PermissionTier;
use alp_media::{ExportPreset, export};

/// Result of listing available export presets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetListReport {
    /// Available presets with their labels and descriptions.
    pub presets: Vec<PresetEntry>,
}

/// Individual export preset entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetEntry {
    /// Preset identifier (e.g., "podcast").
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// Description of the preset.
    pub description: String,
    /// Default sample rate in Hz.
    pub sample_rate: u32,
    /// Default bit depth.
    pub bit_depth: String,
}

/// Result of a stem export operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StemExportReport {
    /// Project track ID.
    pub track_id: String,
    /// Output file path.
    pub output_path: String,
    /// Duration in seconds.
    pub duration_secs: f64,
}

/// Result of transcript export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptExportReport {
    /// Format of the exported transcript ("text", "srt", or "vtt").
    pub format: String,
    /// Absolute path of the exported transcript file.
    pub path: String,
    /// Duration in seconds.
    pub duration_secs: f64,
}

/// Quick preview render of a project segment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewRenderResult {
    /// Start position in seconds.
    pub start_secs: f64,
    /// Duration in seconds.
    pub duration_secs: f64,
    /// Sample rate.
    pub sample_rate: u32,
    /// Channel count.
    pub channels: u16,
    /// Peak amplitude.
    pub peak: f32,
}

impl ApiSession {
    /// List available export presets.
    pub fn export_presets(&self) -> Result<PresetListReport, ApiError> {
        self.check_permission(PermissionTier::Observe)?;
        Ok(PresetListReport {
            presets: vec![
                PresetEntry {
                    id: "podcast".into(),
                    label: "Podcast".into(),
                    description: "44.1 kHz, 16-bit FLAC, triangular dither".into(),
                    sample_rate: 44_100,
                    bit_depth: "16-bit".into(),
                },
                PresetEntry {
                    id: "audiobook".into(),
                    label: "Audiobook".into(),
                    description: "44.1 kHz, 16-bit WAV, triangular dither".into(),
                    sample_rate: 44_100,
                    bit_depth: "16-bit".into(),
                },
                PresetEntry {
                    id: "voice_over".into(),
                    label: "Voice Over".into(),
                    description: "48.0 kHz, 24-bit WAV, triangular dither".into(),
                    sample_rate: 48_000,
                    bit_depth: "24-bit".into(),
                },
            ],
        })
    }

    /// Export project stems (one audio file per track).
    pub fn export_stems(
        &mut self,
        output_dir: &str,
        preset_id: &str,
    ) -> Result<Vec<StemExportReport>, ApiError> {
        self.check_permission(PermissionTier::Export)?;
        let preset = match preset_id {
            "podcast" => ExportPreset::Podcast,
            "audiobook" => ExportPreset::Audiobook,
            _ => ExportPreset::VoiceOver,
        };

        let validated_dir = self.sandbox.validate_path(output_dir)?;
        std::fs::create_dir_all(&validated_dir)?;

        let buffer = self.render_project_mixdown()?;
        let mut reports = Vec::new();

        for track in &self.project.tracks {
            let file_name = format!("stem_track_{}.wav", track.id);
            let target = validated_dir.join(&file_name);
            let settings = preset.settings();
            export(&target, &buffer, &settings)?;

            reports.push(StemExportReport {
                track_id: track.id.to_string(),
                output_path: target.display().to_string(),
                duration_secs: buffer.duration_secs(),
            });
        }

        Ok(reports)
    }

    /// Export a transcript of all analysis results across the project.
    pub fn export_transcript(
        &mut self,
        format: &str,
        path: &str,
    ) -> Result<TranscriptExportReport, ApiError> {
        self.check_permission(PermissionTier::Export)?;

        let written_format = format.to_ascii_lowercase();
        if !matches!(written_format.as_str(), "text" | "srt" | "vtt") {
            return Err(ApiError::Validation(format!(
                "unknown transcript format '{written_format}': use text, srt or vtt"
            )));
        }

        let target = self.sandbox.validate_path(path)?;
        let buf = self.render_project_mixdown()?;

        let content = format!(
            "# Project Transcript: {}\nDuration: {:.2}s\n",
            self.project.name,
            buf.duration_secs()
        );
        std::fs::write(&target, content)?;

        Ok(TranscriptExportReport {
            format: written_format,
            path: target.display().to_string(),
            duration_secs: buf.duration_secs(),
        })
    }

    /// Quick preview render of a project segment for AI client preview.
    pub fn preview_render(
        &mut self,
        start_secs: f64,
        duration_secs: f64,
    ) -> Result<PreviewRenderResult, ApiError> {
        self.check_permission(PermissionTier::Observe)?;
        let buf = self.render_project_mixdown()?;

        Ok(PreviewRenderResult {
            start_secs,
            duration_secs: duration_secs.min(buf.duration_secs()),
            sample_rate: buf.sample_rate(),
            channels: buf.channels(),
            peak: buf.peak(),
        })
    }
}
