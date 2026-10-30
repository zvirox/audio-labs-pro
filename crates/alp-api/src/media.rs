//! Media tool methods for `ApiSession`: project source listing and sandboxed
//! audio import (ADR 0027). Imports ride the undoable `alp-core` command
//! stack (`AddSource`), re-encode decoded audio to Float32 WAV in the
//! project `media/` directory, and mirror the editor's own storage
//! conventions so projects stay interchangeable across surfaces.

use std::path::PathBuf;

use alp_core::{AddSource, Source, SourceId, frames_to_secs};
use alp_media::{WavBitDepth, decode, write_wav};
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::session::ApiSession;
use crate::types::PermissionTier;

/// One project media source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaSummary {
    /// Stable source identifier.
    pub id: String,
    /// Display name recorded at import.
    pub name: String,
    /// Path relative to the project root (`media/...`).
    pub relative_path: String,
    /// Native sample rate of the file in Hz.
    pub sample_rate: u32,
    /// Native channel count.
    pub channels: u16,
    /// Native length in frames.
    pub frames: u64,
    /// Duration in seconds at the source's own sample rate.
    pub duration_secs: f64,
    /// Largest absolute sample value measured at import.
    pub peak: f32,
    /// Whether the media file is present on disk.
    pub file_exists: bool,
}

/// Result of the `media_list` tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaListReport {
    /// Sources in project order.
    pub media: Vec<MediaSummary>,
}

/// Result of the `media_import` tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaImportReport {
    /// Stable source identifier (also used by `clip_add` and `media_list`).
    pub source_id: String,
    /// Display name recorded for the source.
    pub name: String,
    /// Path relative to the project root (`media/{source_id}-{slug}.wav`).
    pub relative_path: String,
    /// Absolute path of the stored media file.
    pub path: String,
    /// Duration in seconds at the source's own sample rate.
    pub duration_secs: f64,
    /// Native sample rate of the stored file in Hz.
    pub sample_rate: u32,
    /// Channel count of the stored file.
    pub channels: u16,
    /// Largest absolute sample value of the decoded audio.
    pub peak: f32,
}

/// Result of the `clip_add` tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClipPlacementReport {
    /// Stable clip identifier.
    pub clip_id: String,
    /// Owning track identifier.
    pub track_id: String,
    /// Source the clip reads from.
    pub source_id: String,
    /// Timeline start in seconds (project rate).
    pub start_secs: f64,
    /// Clip length in seconds (source rate).
    pub duration_secs: f64,
    /// Offset into the source in seconds.
    pub source_offset_secs: f64,
    /// Linear clip gain after clamping (`1.0` is unity).
    pub gain: f32,
}

/// Filesystem-safe slug for stored media names (mirrors the UI's
/// `safe_file_stem`: ASCII alphanumerics plus `-`/`_`, lowercase, bounded).
fn slug(name: &str) -> String {
    let mapped: String = name
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let collapsed: String = mapped
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let trimmed = collapsed.trim_matches('-');
    if trimmed.is_empty() {
        "audio".to_owned()
    } else {
        trimmed.chars().take(48).collect()
    }
}

impl ApiSession {
    /// List project media sources (Phase 10 `media_list`).
    pub fn media_list(&self) -> Result<MediaListReport, ApiError> {
        self.check_permission(PermissionTier::Observe)?;
        let media = self
            .project
            .sources
            .iter()
            .map(|source| {
                let on_disk = self
                    .paths
                    .resolve_relative(&source.relative_path)
                    .map(|path| path.is_file())
                    .unwrap_or(false);
                MediaSummary {
                    id: source.id.to_string(),
                    name: source.name.clone(),
                    relative_path: source.relative_path.clone(),
                    sample_rate: source.sample_rate,
                    channels: source.channels,
                    frames: source.frames,
                    duration_secs: frames_to_secs(source.frames, source.sample_rate),
                    peak: source.peak,
                    file_exists: on_disk,
                }
            })
            .collect();
        Ok(MediaListReport { media })
    }

    /// Import a sandboxed audio file into the project media directory and
    /// register it as a source (Phase 10 `media_import`). The file is decoded
    /// (proving it is audio), re-encoded as Float32 WAV under a fresh source
    /// id so it can never overwrite existing media, peak-measured, and added
    /// with the undoable `AddSource` command.
    pub fn media_import(
        &mut self,
        path: &str,
        name: Option<&str>,
    ) -> Result<MediaImportReport, ApiError> {
        self.check_permission(PermissionTier::Edit)?;

        let input = self.sandbox.validate_path(path)?;
        let buffer = decode(&input)?;

        let display_name = name
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(String::from)
            .unwrap_or_else(|| {
                input
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Imported audio".to_owned())
            });

        let source_id = SourceId::new();
        let file_name = format!("{source_id}-{}.wav", slug(&display_name));
        let target: PathBuf = self
            .sandbox
            .validate_path(self.paths.media_dir().join(&file_name))?;
        write_wav(&target, &buffer, WavBitDepth::Float32)?;
        let relative_path = format!("media/{file_name}");

        let source = Source::new(
            source_id,
            display_name.clone(),
            relative_path.clone(),
            buffer.sample_rate(),
            buffer.channels(),
            buffer.frames(),
        )
        .map_err(ApiError::from)?
        .with_peak(buffer.peak());

        let index = self.project.sources.len();
        self.execute_command("media_import", Box::new(AddSource::new(source, index)))?;

        Ok(MediaImportReport {
            source_id: source_id.to_string(),
            name: display_name,
            relative_path,
            path: target.display().to_string(),
            duration_secs: frames_to_secs(buffer.frames(), buffer.sample_rate()),
            sample_rate: buffer.sample_rate(),
            channels: buffer.channels(),
            peak: buffer.peak(),
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
        let project = Project::new("Media Test").expect("project");
        let sandbox = WorkspaceSandbox::new([dir]).expect("sandbox");
        let audit = AuditLogger::new(dir.join("audit.jsonl")).expect("audit");
        ApiSession::new(project, proj_paths, tier, sandbox, audit, "agent")
            .expect("session")
            .with_voice_store_dir(dir.join("voices"))
    }

    #[test]
    fn media_import_registers_an_undoable_source() {
        let dir = tempdir().expect("tempdir");
        let mut session = session_in(dir.path(), PermissionTier::Edit);

        // Produce real audio with the voice toolset, then import it - the
        // recommended generate-to-timeline chain (ADR 0026 + 0027).
        let tts = session
            .tts_generate("Media import check", "aura", None)
            .expect("generate");
        let imported = session
            .media_import(&tts.path, Some("Agent Take"))
            .expect("import");

        assert!(
            imported.relative_path.starts_with("media/"),
            "imports live in the media dir: {}",
            imported.relative_path
        );
        assert!(imported.relative_path.ends_with(".wav"));
        assert!(
            std::path::Path::new(&imported.path).is_file(),
            "wavs are stored"
        );
        assert_eq!(imported.name, "Agent Take");
        assert!(imported.duration_secs > 0.0);
        assert!(imported.peak > 0.0);

        let list = session.media_list().expect("list");
        assert_eq!(list.media.len(), 1);
        assert_eq!(list.media[0].id, imported.source_id);
        assert!(list.media[0].file_exists, "the stored file exists");
        assert_eq!(list.media[0].relative_path, imported.relative_path);

        // Undoable like every other Edit-tool mutation.
        assert!(session.undo().expect("undo"), "import is undoable");
        assert!(session.media_list().expect("list").media.is_empty());
        assert!(session.redo().expect("redo"), "import is redoable");
        assert_eq!(session.media_list().expect("list").media.len(), 1);
    }

    #[test]
    fn media_import_never_overwrites_and_defaults_names() {
        let dir = tempdir().expect("tempdir");
        let mut session = session_in(dir.path(), PermissionTier::Edit);
        let tts = session
            .tts_generate("Name default check", "aura", None)
            .expect("generate");

        let first = session.media_import(&tts.path, None).expect("first import");
        let first_name = std::path::Path::new(&first.path)
            .file_name()
            .expect("file name")
            .to_string_lossy()
            .into_owned();
        let second = session
            .media_import(&tts.path, None)
            .expect("second import of the same file");
        let second_name = std::path::Path::new(&second.path)
            .file_name()
            .expect("file name")
            .to_string_lossy()
            .into_owned();

        assert_ne!(first_name, second_name, "each import gets a fresh file");
        assert!(first.relative_path != second.relative_path);
        // Name defaults to the file stem of the imported file.
        let expected_stem = std::path::Path::new(&tts.path)
            .file_stem()
            .expect("stem")
            .to_string_lossy()
            .into_owned();
        assert_eq!(
            first.name, expected_stem,
            "name defaults to the imported stem"
        );
    }

    #[test]
    fn media_import_rejects_non_audio_and_escape_paths() {
        let dir = tempdir().expect("tempdir");
        let mut session = session_in(dir.path(), PermissionTier::Edit);

        let text_file = dir.path().join("notes.txt");
        std::fs::write(&text_file, b"definitely not audio").expect("write text");
        let not_audio = session
            .media_import(text_file.to_str().expect("utf8 path"), None)
            .expect_err("text is not audio");
        assert!(matches!(not_audio, ApiError::Media(_)));

        let escape = session
            .media_import("../escape.wav", None)
            .expect_err("traversal");
        assert!(matches!(escape, ApiError::SandboxEscape(_)));
    }

    #[test]
    fn observe_tier_cannot_import() {
        let dir = tempdir().expect("tempdir");
        let mut session = session_in(dir.path(), PermissionTier::Observe);
        let error = session
            .media_import("somewhere.wav", None)
            .expect_err("tier");
        assert!(matches!(error, ApiError::PermissionDenied(_)));
    }

    #[test]
    fn clip_add_places_full_source_defaults_and_is_undoable() {
        let dir = tempdir().expect("tempdir");
        let mut session = session_in(dir.path(), PermissionTier::Edit);
        let track_id = session.add_track("Narration").expect("add track");

        let tts = session
            .tts_generate("Clip placement check", "aura", None)
            .expect("generate");
        let imported = session
            .media_import(&tts.path, Some("Take"))
            .expect("import");

        let placed = session
            .clip_add(&track_id, &imported.source_id, 0.0, None, None, None)
            .expect("place");
        assert_eq!(placed.track_id, track_id);
        assert_eq!(placed.source_id, imported.source_id);
        assert_eq!(placed.start_secs, 0.0);
        assert_eq!(placed.gain, 1.0, "default unity gain");
        // Default duration covers the whole remaining source.
        assert!((placed.duration_secs - imported.duration_secs).abs() < 0.01);

        // Three infos stay coherent: source registered, clip counted.
        assert_eq!(session.media_list().expect("list").media.len(), 1);
        assert_eq!(session.project_info().expect("info").clip_count, 1);

        // Undo removes the clip but keeps the source (undo bounds are per command).
        assert!(session.undo().expect("undo"), "clip placement is undoable");
        assert_eq!(session.project_info().expect("info").clip_count, 0);
        assert_eq!(session.media_list().expect("list").media.len(), 1);

        // Redo restores the placement, and overlaps stay legal at the core
        // layer (the mix engine sums them).
        assert!(session.redo().expect("redo"), "clip placement is redoable");
        session
            .clip_add(&track_id, &imported.source_id, 0.0, None, None, None)
            .expect("overlapping placement is legal");
        assert_eq!(session.project_info().expect("info").clip_count, 2);
    }

    #[test]
    fn clip_add_validates_arguments_and_ids() {
        let dir = tempdir().expect("tempdir");
        let mut session = session_in(dir.path(), PermissionTier::Edit);
        let track_id = session.add_track("Narration").expect("add track");
        let tts = session
            .tts_generate("Validation check", "aura", None)
            .expect("generate");
        let imported = session.media_import(&tts.path, None).expect("import");

        let unknown_source = session
            .clip_add(&track_id, "src_missing", 0.0, None, None, None)
            .expect_err("unknown source");
        assert!(matches!(unknown_source, ApiError::NotFound(_)));

        let unknown_track = session
            .clip_add("trk_missing", &imported.source_id, 0.0, None, None, None)
            .expect_err("unknown track");
        assert!(
            matches!(unknown_track, ApiError::NotFound(_)),
            "track errors are not-found errors: {unknown_track}"
        );

        let negative = session
            .clip_add(&track_id, &imported.source_id, -1.0, None, None, None)
            .expect_err("negative start");
        assert!(matches!(negative, ApiError::Validation(_)));

        let zero_duration = session
            .clip_add(&track_id, &imported.source_id, 0.0, Some(0.0), None, None)
            .expect_err("zero duration");
        assert!(matches!(zero_duration, ApiError::Validation(_)));

        // Gain is clamped into [0.0, 4.0], not rejected.
        let loud = session
            .clip_add(&track_id, &imported.source_id, 0.0, None, None, Some(50.0))
            .expect("loud placement");
        assert_eq!(loud.gain, 4.0, "gain is clamped to +4.0 linear");
    }
}
