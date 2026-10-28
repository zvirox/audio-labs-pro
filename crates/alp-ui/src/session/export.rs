//! Export settings, presets and scene snapshots for `EditSession`.

use std::path::Path;

use alp_engine::MixScene;
use alp_media::{AudioMetadata, ExportPreset, ExportSettings};

use super::EditSession;

impl EditSession {
    /// Snapshot for a background render.
    pub fn scene(&self) -> Result<MixScene, String> {
        MixScene::from_project(&self.project, &self.audio).map_err(|error| error.to_string())
    }

    /// Record an export path queued for the background worker.
    pub fn set_export_queued(&mut self, path: &Path) {
        self.last_export = Some(path.display().to_string());
        self.status = format!("Queued export {}", path.display());
    }

    /// Default settings for the export button.
    #[must_use]
    pub fn export_settings() -> ExportSettings {
        ExportSettings {
            format: alp_media::ExportFormat::Wav,
            bit_depth: alp_media::WavBitDepth::Int24,
            sample_rate: Some(48_000),
            dither: alp_media::DitherMode::Triangular,
            metadata: AudioMetadata {
                title: Some("Phase 4 mix".into()),
                artist: Some("Audio Labs Pro".into()),
                album: Some("Demo session".into()),
                comment: None,
            },
        }
    }

    /// Preconfigured export settings for an industry preset.
    #[must_use]
    #[allow(dead_code)]
    pub fn export_preset_settings(preset: ExportPreset) -> ExportSettings {
        preset.settings()
    }
}
