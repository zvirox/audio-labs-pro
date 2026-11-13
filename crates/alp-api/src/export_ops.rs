//! Export operations (presets, stems, transcript, preview) for `ApiSession`.
//!
//! Each export produces the artefact it names: stems are rendered per track
//! (not the master copied N times), transcripts contain the actual words, and
//! a preview render is a real audio slice written to disk.

use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::session::ApiSession;
use crate::types::PermissionTier;
use alp_media::{ExportPreset, export};
use alp_voice::Transcript;

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
    /// Track name at export time.
    pub track_name: String,
    /// Output file path.
    pub output_path: String,
    /// Duration in seconds.
    pub duration_secs: f64,
    /// Sample peak of the stem in dBFS (proves the stem carries that track).
    pub peak_dbfs: f32,
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
    /// Engine that produced the words.
    pub engine: String,
    /// Number of segments written.
    pub segments: usize,
    /// The transcript text itself, so callers need not re-read the file.
    pub text: String,
}

/// Quick preview render of a project segment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewRenderResult {
    /// Start position in seconds, clamped to the project duration.
    pub start_secs: f64,
    /// Duration of the preview in seconds.
    pub duration_secs: f64,
    /// Sample rate.
    pub sample_rate: u32,
    /// Channel count.
    pub channels: u16,
    /// Peak amplitude of the preview slice (linear).
    pub peak: f32,
    /// Path of the written preview WAV.
    pub preview_path: String,
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

    /// Export project stems: one audio file per track containing that track's
    /// own audio, not the master mix.
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
        if self.project.tracks.is_empty() {
            return Err(ApiError::Validation(
                "cannot export stems: the project has no tracks".into(),
            ));
        }

        let validated_dir = self.sandbox.validate_path(output_dir)?;
        std::fs::create_dir_all(&validated_dir)?;
        let has_synthetic = self.project_has_synthetic_sources();
        let track_ids: Vec<(String, String)> = self
            .project
            .tracks
            .iter()
            .map(|track| (track.id.to_string(), track.name.clone()))
            .collect();

        let mut reports = Vec::new();
        for (index, (track_id, track_name)) in track_ids.iter().enumerate() {
            let (_, buffer) = self.render_track_mixdown(track_id)?;
            let file_name = format!("stem_{:02}_{}.wav", index + 1, sanitize_name(track_name));
            let target = validated_dir.join(&file_name);
            let mut settings = preset.settings();
            if has_synthetic {
                settings.metadata.synthetic = true;
                settings.metadata.watermark = Some("Audio Labs Pro (Synthetic Audio Stem)".into());
            }
            export(&target, &buffer, &settings)?;
            let peak = buffer.peak().max(1e-6);
            reports.push(StemExportReport {
                track_id: track_id.clone(),
                track_name: track_name.clone(),
                output_path: target.display().to_string(),
                duration_secs: buffer.duration_secs(),
                peak_dbfs: 20.0 * peak.log10(),
            });
        }

        self.record_audit("export_stems", "ok", self.revision);
        Ok(reports)
    }

    /// Export a transcript of the project audio. The words come from the real
    /// transcription path (daemon STT when available, built-in segmenter
    /// otherwise), and the report names the engine that ran.
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
        if buf.samples().iter().all(|s| s.abs() < 1e-6) {
            return Err(ApiError::Validation(
                "cannot export a transcript: the project renders to digital silence".into(),
            ));
        }
        let rate = buf.sample_rate();

        let mut engine = String::from("builtin");
        let segments = crate::voice::transcribe_daemon_first(&buf, &mut engine)?;
        let transcript = Transcript { segments };
        let content = match written_format.as_str() {
            "srt" => transcript.to_srt(rate),
            "vtt" => transcript.to_vtt(rate),
            _ => transcript.to_plain_text(),
        };
        std::fs::write(&target, &content)?;

        self.record_audit("export_transcript", "ok", self.revision);
        Ok(TranscriptExportReport {
            format: written_format,
            path: target.display().to_string(),
            duration_secs: buf.duration_secs(),
            engine,
            segments: transcript.segments.len(),
            text: content,
        })
    }

    /// Quick preview render of a project segment: the slice is written as a
    /// WAV in the project media directory and its own peak is reported.
    pub fn preview_render(
        &mut self,
        start_secs: f64,
        duration_secs: f64,
    ) -> Result<PreviewRenderResult, ApiError> {
        self.check_permission(PermissionTier::Observe)?;
        if !start_secs.is_finite() || start_secs < 0.0 {
            return Err(ApiError::Validation(format!(
                "invalid start {start_secs}: must be a non-negative number of seconds"
            )));
        }
        if !duration_secs.is_finite() || duration_secs <= 0.0 {
            return Err(ApiError::Validation(format!(
                "invalid duration {duration_secs}: must be a positive number of seconds"
            )));
        }

        let buf = self.render_project_mixdown()?;
        let rate = buf.sample_rate();
        let channels = usize::from(buf.channels().max(1));
        let frames = buf.samples().len() / channels;
        let start_frame = ((start_secs * f64::from(rate)) as usize).min(frames);
        let want = (duration_secs * f64::from(rate)) as usize;
        let end_frame = (start_frame + want.max(1)).min(frames);
        if end_frame <= start_frame {
            return Err(ApiError::Validation(format!(
                "preview range {start_secs:.3}s + {duration_secs:.3}s is past the end of the project ({:.3}s)",
                buf.duration_secs()
            )));
        }

        let slice = buf.samples()[start_frame * channels..end_frame * channels].to_vec();
        let preview_len = slice.len() / channels;
        let preview = alp_media::PcmBuffer::new(rate, buf.channels(), slice)?;

        let file_name = format!(
            "preview_{:08}_{:08}.wav",
            (start_secs * 1000.0) as u64,
            (preview_len as f64 / f64::from(rate) * 1000.0) as u64
        );
        let target = self
            .sandbox
            .validate_path(self.paths.media_dir().join(file_name))?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut settings = ExportPreset::VoiceOver.settings();
        if self.project_has_synthetic_sources() {
            settings.metadata.synthetic = true;
            settings.metadata.watermark = Some("Audio Labs Pro (Synthetic Audio Preview)".into());
        }
        export(&target, &preview, &settings)?;

        self.record_audit("preview_render", "ok", self.revision);
        Ok(PreviewRenderResult {
            start_secs: start_frame as f64 / f64::from(rate),
            duration_secs: preview_len as f64 / f64::from(rate),
            sample_rate: rate,
            channels: buf.channels(),
            peak: preview.peak(),
            preview_path: target.display().to_string(),
        })
    }

    /// Whether any imported source was written by the synthetic voice path.
    /// The tag lives in the container metadata, so this reads the source bytes
    /// and looks for the marker the export path writes.
    pub(crate) fn project_has_synthetic_sources(&self) -> bool {
        self.project.sources.iter().any(|source| {
            self.paths
                .resolve_relative(&source.relative_path)
                .ok()
                .and_then(|path| std::fs::read(path).ok())
                .is_some_and(|bytes| {
                    let content = String::from_utf8_lossy(&bytes);
                    content.contains("synthetic_voice: true")
                        || content.contains("SYNTHETIC_VOICE=true")
                })
        })
    }
}

/// Reduce a track name to a filesystem-safe stem.
fn sanitize_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('_').to_owned();
    if trimmed.is_empty() {
        "track".to_owned()
    } else {
        trimmed.chars().take(40).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_name_keeps_a_readable_stem() {
        assert_eq!(sanitize_name("Lead Vocal"), "lead_vocal");
        assert_eq!(sanitize_name("  "), "track");
        assert_eq!(sanitize_name("A/B:C"), "a_b_c");
    }
}
