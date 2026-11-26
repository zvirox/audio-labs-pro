//! Audiobook mastering bridge for the desktop UI.
//!
//! Decodes chapter/master audio from disk and delegates to the pure analysis
//! and muxing implementations in `alp_api` and `alp_media`.

use std::path::Path;

use alp_api::{AcxChapterInput, AcxQaReport, analyze_acx_chapters};
use alp_media::{
    ExportSettings, M4bChapter, M4bCover, M4bExportReport, M4bMetadata, decode, export_m4b,
};
use serde::{Deserialize, Serialize};

/// One chapter audio file to analyse.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcxChapterFileDto {
    /// Chapter title.
    pub title: String,
    /// Path to a decoded audio file.
    pub audio_path: String,
}

/// Request for a native M4B export.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct M4bExportRequestDto {
    /// Destination `.m4b` path.
    pub output_path: String,
    /// Full-book audio rendered before chapter markers are applied.
    pub master_audio_path: String,
    /// Chapter markers with titles and timing.
    pub chapters: Vec<M4bChapterDto>,
    /// Optional cover artwork path (JPEG or PNG).
    pub cover_path: Option<String>,
    /// Book metadata.
    pub metadata: M4bMetadataDto,
}

/// Chapter marker in an M4B request.
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

/// Analyse chapter audio files against ACX targets.
pub fn analyze_acx_files(chapters: &[AcxChapterFileDto]) -> Result<AcxQaReport, String> {
    let mut decoded = Vec::with_capacity(chapters.len());
    for chapter in chapters {
        let pcm = decode(Path::new(&chapter.audio_path))
            .map_err(|error| format!("failed to decode {}: {error}", chapter.audio_path))?;
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

/// Export a rendered master audio file as a chaptered M4B audiobook.
pub fn export_audiobook_m4b(request: &M4bExportRequestDto) -> Result<M4bExportReport, String> {
    let pcm = decode(Path::new(&request.master_audio_path))
        .map_err(|error| format!("failed to decode {}: {error}", request.master_audio_path))?;
    let chapters: Vec<M4bChapter> = request
        .chapters
        .iter()
        .map(|chapter| M4bChapter {
            title: chapter.title.clone(),
            start_secs: chapter.start_secs,
            duration_secs: chapter.duration_secs,
        })
        .collect();
    let cover = if let Some(path) = &request.cover_path {
        let data =
            std::fs::read(path).map_err(|error| format!("failed to read cover {path}: {error}"))?;
        let format = if data.starts_with(&[0x89, b'P', b'N', b'G']) {
            alp_media::CoverFormat::Png
        } else {
            alp_media::CoverFormat::Jpeg
        };
        Some(M4bCover { format, data })
    } else {
        None
    };
    let metadata = M4bMetadata {
        title: request.metadata.title.clone(),
        narrator: request.metadata.narrator.clone(),
        series: request.metadata.series.clone(),
        series_number: request.metadata.series_number,
        comment: request.metadata.comment.clone(),
    };
    export_m4b(
        Path::new(&request.output_path),
        &pcm,
        &ExportSettings::preset(alp_media::ExportPreset::Audiobook),
        &chapters,
        cover.as_ref(),
        &metadata,
    )
    .map_err(|error| format!("M4B export failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acx_files_reports_decode_errors() {
        let result = analyze_acx_files(&[AcxChapterFileDto {
            title: "missing".into(),
            audio_path: "/definitely/not/here.wav".into(),
        }]);
        assert!(result.is_err());
    }
}
