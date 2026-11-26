//! Automation wrappers for studio features that previously existed only in
//! the desktop session layer.
//!
//! Every wrapper reuses an existing pure implementation (`alp_dsp`,
//! `analyze_acx_chapters`, `alp_media::export_m4b`) or an existing `alp_core`
//! command. Nothing here touches the real-time audio callback.

use alp_core::{
    ClipId, FrameRange, MarkerId, RemoveMarker, SetLoopRange, TrimClip, secs_to_frames,
};
use alp_dsp::{Coefficients, FilterType};
use alp_media::{
    CoverFormat, ExportSettings, M4bChapter, M4bCover, M4bExportReport, M4bMetadata, decode,
    export_m4b,
};
use serde::{Deserialize, Serialize};

use crate::{
    AcxChapterInput, AcxQaReport, ApiError, PermissionTier, analyze_acx_chapters,
    session::ApiSession,
};

/// One EQ band supplied to `effects_response`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EqBandDto {
    /// Filter shape (`highpass`, `lowpass`, `peaking`, `lowshelf`, `highshelf`,
    /// `notch`, `bandpass`); unknown names fall back to peaking.
    pub filter_type: String,
    /// Centre or cutoff frequency in Hz.
    pub frequency_hz: f32,
    /// Quality factor.
    pub q: f32,
    /// Peaking/shelving gain in dB.
    pub gain_db: f32,
    /// Whether the band contributes to the response.
    pub enabled: bool,
}

/// Sampled complex response of an EQ band chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EqResponseDto {
    /// Log-spaced analysis frequencies in Hz.
    pub frequencies_hz: Vec<f32>,
    /// Combined magnitude in dB at each frequency.
    pub magnitude_db: Vec<f32>,
    /// Combined unwrapped phase in degrees at each frequency.
    pub phase_deg: Vec<f32>,
}

/// One chapter audio file supplied to `acx_qa_analyze`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcxChapterFileDto {
    /// Display title used in the report.
    pub title: String,
    /// Workspace path to a decodable audio file.
    pub audio_path: String,
}

/// Chapter marker for a native M4B export.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct M4bChapterDto {
    /// Chapter title.
    pub title: String,
    /// Start time in seconds.
    pub start_secs: f64,
    /// Duration in seconds.
    pub duration_secs: f64,
}

/// Serializable subset of M4B metadata.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct M4bMetadataDto {
    /// Book title.
    pub title: String,
    /// Narrator.
    pub narrator: String,
    /// Series name.
    pub series: String,
    /// Series number.
    pub series_number: Option<u32>,
    /// Description.
    pub comment: String,
}

/// Request for a native chaptered M4B export.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct M4bExportRequestDto {
    /// Destination `.m4b` path inside the workspace.
    pub output_path: String,
    /// Full-book audio rendered before chapter markers are applied.
    pub master_audio_path: String,
    /// Chapter markers with titles and timing.
    pub chapters: Vec<M4bChapterDto>,
    /// Optional cover artwork path (JPEG or PNG) inside the workspace.
    pub cover_path: Option<String>,
    /// Book metadata.
    pub metadata: M4bMetadataDto,
}

fn filter_type(name: &str) -> FilterType {
    match name.to_lowercase().as_str() {
        "highpass" | "high_pass" | "hp" => FilterType::HighPass,
        "lowpass" | "low_pass" | "lp" => FilterType::LowPass,
        "lowshelf" | "low_shelf" => FilterType::LowShelf,
        "highshelf" | "high_shelf" => FilterType::HighShelf,
        "notch" => FilterType::Notch,
        "bandpass" | "band_pass" => FilterType::BandPass,
        _ => FilterType::Peaking,
    }
}

/// Compute the combined response of enabled bands on a log frequency grid.
#[must_use]
pub fn compute_eq_response(bands: &[EqBandDto], sample_rate: u32, points: usize) -> EqResponseDto {
    let points = points.clamp(16, 512);
    let nyquist = (sample_rate as f32 * 0.5).max(1.0);
    let max_hz = nyquist * 0.99;
    let min_hz = 20.0f32.min(max_hz);
    let ratio = (max_hz / min_hz).ln() / (points.saturating_sub(1).max(1) as f32);

    let coefficients: Vec<_> = bands
        .iter()
        .filter(|band| band.enabled)
        .map(|band| {
            Coefficients::design(
                filter_type(&band.filter_type),
                sample_rate,
                band.frequency_hz,
                band.q,
                band.gain_db,
            )
        })
        .collect();

    let mut frequencies_hz = Vec::with_capacity(points);
    let mut magnitude_db = Vec::with_capacity(points);
    let mut phase_deg = Vec::with_capacity(points);

    for index in 0..points {
        let frequency = min_hz * (index as f32 * ratio).exp();
        frequencies_hz.push(frequency);
        let mut magnitude = 1.0f32;
        let mut phase = 0.0f32;
        for coefficient in &coefficients {
            let (db, degrees) = coefficient.response(sample_rate, frequency);
            magnitude *= 10.0f32.powf(db / 20.0);
            phase += degrees;
        }
        magnitude_db.push((20.0 * magnitude.max(1e-6).log10()).clamp(-120.0, 24.0));
        phase_deg.push(phase.clamp(-360.0, 360.0));
    }

    EqResponseDto {
        frequencies_hz,
        magnitude_db,
        phase_deg,
    }
}

impl ApiSession {
    /// Analyse chapter audio files against the ACX publication targets.
    pub fn acx_qa_analyze(&self, chapters: &[AcxChapterFileDto]) -> Result<AcxQaReport, ApiError> {
        self.check_permission(PermissionTier::Observe)?;
        let mut decoded = Vec::with_capacity(chapters.len());
        for chapter in chapters {
            let path = self.sandbox.validate_path(&chapter.audio_path)?;
            let pcm = decode(&path)?;
            decoded.push((chapter.title.as_str(), pcm));
        }
        let inputs: Vec<AcxChapterInput<'_>> = decoded
            .iter()
            .map(|(title, pcm)| AcxChapterInput {
                title,
                samples: pcm.samples(),
                sample_rate: pcm.sample_rate(),
                channels: pcm.channels(),
            })
            .collect();
        Ok(analyze_acx_chapters(&inputs))
    }

    /// Export a rendered master as a chaptered M4B audiobook.
    pub fn audiobook_export_m4b(
        &mut self,
        request: &M4bExportRequestDto,
    ) -> Result<M4bExportReport, ApiError> {
        self.check_permission(PermissionTier::Export)?;
        let master = self.sandbox.validate_path(&request.master_audio_path)?;
        let output = self.sandbox.validate_path(&request.output_path)?;
        let cover_path = request
            .cover_path
            .as_deref()
            .map(|path| self.sandbox.validate_path(path))
            .transpose()?;
        let pcm = decode(&master)?;
        let cover = if let Some(path) = &cover_path {
            let data = std::fs::read(path)?;
            let format = if data.starts_with(&[0x89, b'P', b'N', b'G']) {
                CoverFormat::Png
            } else {
                CoverFormat::Jpeg
            };
            Some(M4bCover { format, data })
        } else {
            None
        };
        let chapters: Vec<M4bChapter> = request
            .chapters
            .iter()
            .map(|chapter| M4bChapter {
                title: chapter.title.clone(),
                start_secs: chapter.start_secs,
                duration_secs: chapter.duration_secs,
            })
            .collect();
        let metadata = M4bMetadata {
            title: request.metadata.title.clone(),
            narrator: request.metadata.narrator.clone(),
            series: request.metadata.series.clone(),
            series_number: request.metadata.series_number,
            comment: request.metadata.comment.clone(),
        };
        Ok(export_m4b(
            &output,
            &pcm,
            &ExportSettings::preset(alp_media::ExportPreset::Audiobook),
            &chapters,
            cover.as_ref(),
            &metadata,
        )?)
    }

    /// Remove a timeline marker by id.
    pub fn marker_remove(&mut self, marker_id: &str) -> Result<(), ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        let id: MarkerId = marker_id
            .parse()
            .map_err(|_| ApiError::Validation(format!("invalid marker id '{marker_id}'")))?;
        if self.project.marker(id).is_none() {
            return Err(ApiError::NotFound(format!("marker '{marker_id}'")));
        }
        self.execute_command("marker_remove", Box::new(RemoveMarker::new(id)))
    }

    /// Set or clear the project loop range.
    pub fn loop_range_set(
        &mut self,
        start_secs: Option<f64>,
        end_secs: Option<f64>,
    ) -> Result<(), ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        let range = match (start_secs, end_secs) {
            (Some(start), Some(end)) => {
                let rate = self.project.sample_rate;
                let start_frame = secs_to_frames(start.max(0.0), rate);
                let end_frame = secs_to_frames(end.max(0.0), rate);
                Some(FrameRange::new(start_frame, end_frame)?)
            }
            (None, None) => None,
            _ => {
                return Err(ApiError::Validation(
                    "supply both start_secs and end_secs, or neither to clear".into(),
                ));
            }
        };
        self.execute_command("loop_range_set", Box::new(SetLoopRange::new(range)))
    }

    /// Trim a clip by replacing its source range and timeline start.
    pub fn clip_trim(
        &mut self,
        clip_id: &str,
        source_start_frame: u64,
        source_end_frame: u64,
        timeline_start_frame: u64,
    ) -> Result<(), ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        let id: ClipId = clip_id
            .parse()
            .map_err(|_| ApiError::Validation(format!("invalid clip id '{clip_id}'")))?;
        if self.project.clip(id).is_none() {
            return Err(ApiError::NotFound(format!("clip '{clip_id}'")));
        }
        let range = FrameRange::new(source_start_frame, source_end_frame)?;
        self.execute_command(
            "clip_trim",
            Box::new(TrimClip::new(id, range, timeline_start_frame)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuditLogger, WorkspaceSandbox};
    use alp_core::{AddMarker, Project, ProjectPaths};
    use alp_media::{ExportSettings, PcmBuffer, WavBitDepth, export, write_test_tone};
    use std::f32::consts::PI;
    use tempfile::tempdir;

    fn session_in(dir: &std::path::Path, tier: PermissionTier) -> ApiSession {
        let paths = ProjectPaths::new(dir.join("proj.alp"));
        paths.ensure_layout().expect("layout");
        let sandbox = WorkspaceSandbox::new([dir]).expect("sandbox");
        let audit = AuditLogger::new(dir.join("audit.jsonl")).expect("audit");
        ApiSession::new(
            Project::new("Studio Ops").expect("project"),
            paths,
            tier,
            sandbox,
            audit,
            "agent",
        )
        .expect("session")
    }

    #[test]
    fn eq_response_reports_enabled_bands_only() {
        let band = |enabled| EqBandDto {
            filter_type: "peaking".into(),
            frequency_hz: 1_000.0,
            q: 1.0,
            gain_db: 6.0,
            enabled,
        };
        let active = compute_eq_response(&[band(true)], 48_000, 64);
        let bypassed = compute_eq_response(&[band(false)], 48_000, 64);
        assert_eq!(active.frequencies_hz.len(), 64);
        assert!(active.magnitude_db.iter().any(|db| *db > 0.5));
        assert!(bypassed.magnitude_db.iter().all(|db| db.abs() < 1e-3));
    }

    #[test]
    fn acx_qa_analyze_decodes_workspace_audio() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("chapter.wav");
        let samples: Vec<f32> = (0..48_000)
            .map(|i| 0.3 * (2.0 * PI * 220.0 * i as f32 / 48_000.0).sin())
            .collect();
        export(
            &path,
            &PcmBuffer::new(48_000, 1, samples).expect("pcm"),
            &ExportSettings::wav(WavBitDepth::Int16),
        )
        .expect("write chapter");
        let session = session_in(dir.path(), PermissionTier::Observe);
        let report = session
            .acx_qa_analyze(&[AcxChapterFileDto {
                title: "Chapter 1".into(),
                audio_path: path.display().to_string(),
            }])
            .expect("acx report");
        assert_eq!(report.total_count, 1);
        assert_eq!(report.chapters[0].title, "Chapter 1");
    }

    #[test]
    fn marker_remove_and_loop_range_are_undoable() {
        let dir = tempdir().expect("tempdir");
        let mut session = session_in(dir.path(), PermissionTier::Edit);
        let id = alp_core::MarkerId::new();
        session
            .execute_command(
                "test_marker_add",
                Box::new(AddMarker::new(id, 4_800, "Ch1".to_owned())),
            )
            .expect("add marker");
        session.marker_remove(&id.to_string()).expect("remove marker");
        assert!(session.undo().expect("undo"));
        session.loop_range_set(Some(1.0), Some(2.0)).expect("set loop");
        session.loop_range_set(None, None).expect("clear loop");
    }

    #[test]
    fn m4b_export_writes_a_chaptered_file() {
        let dir = tempdir().expect("tempdir");
        let master = dir.path().join("master.wav");
        write_test_tone(&master, 0.1, 48_000, 1, 440.0, 0.2).expect("master wav");
        let output = dir.path().join("book.m4b");
        let mut session = session_in(dir.path(), PermissionTier::Export);
        let report = session
            .audiobook_export_m4b(&M4bExportRequestDto {
                output_path: output.display().to_string(),
                master_audio_path: master.display().to_string(),
                chapters: vec![M4bChapterDto {
                    title: "Chapter 1".into(),
                    start_secs: 0.0,
                    duration_secs: 0.05,
                }],
                cover_path: None,
                metadata: M4bMetadataDto {
                    title: "Test Book".into(),
                    ..M4bMetadataDto::default()
                },
            })
            .expect("m4b export");
        assert_eq!(report.chapter_count, 1);
        assert!(output.is_file());
    }
}
