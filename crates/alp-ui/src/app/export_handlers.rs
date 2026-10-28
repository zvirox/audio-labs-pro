//! Background audio rendering and preset export handlers.

use alp_core::now_ms;
use alp_media::ExportSettings;

use crate::app::App;
use crate::session::EditSession;

impl App {
    pub(crate) fn export_mix(&mut self) {
        let scene = match self.session.scene() {
            Ok(scene) => scene,
            Err(error) => {
                self.show_toast(format!("Export failed: {error}"));
                return;
            }
        };
        if self.punch_in && self.session.selection().is_empty() {
            self.show_toast("Select a range before punch-in");
            return;
        }
        let Some(dirs) = alp_core::AppDirs::discover().ok() else {
            self.show_toast("No application data directory");
            return;
        };
        let directory = dirs.projects_dir().join("ui-demo").join("renders");
        if let Err(error) = std::fs::create_dir_all(&directory) {
            self.show_toast(format!("Cannot create render folder: {error}"));
            return;
        }
        let path = directory.join(format!("mix-{}.wav", now_ms()));
        let display = path.display().to_string();
        let settings: ExportSettings = EditSession::export_settings();
        let master_gain = self.session.master_gain();
        self.session.set_export_queued(&path);
        self.jobs.spawn("Export mix", move |reporter| {
            reporter.progress(0.05);
            reporter.detail("Rendering edited timeline");
            let mut buffer = alp_media::PcmBuffer::silence(
                scene.sample_rate(),
                scene.channels(),
                scene.total_frames(),
            )
            .map_err(|error| error.to_string())?;
            let mut mixer = alp_engine::Mixer::with_master(scene, master_gain, None, 0.0, 0.0);
            let blocksamples = alp_engine::DEFAULT_BLOCK_FRAMES * usize::from(buffer.channels());
            for block in buffer.samples_mut().chunks_mut(blocksamples) {
                mixer.render_into(block);
            }
            reporter.progress(0.65);
            reporter.detail("Quantizing and writing WAV");
            alp_media::export(&path, &buffer, &settings).map_err(|error| error.to_string())?;
            reporter.progress(1.0);
            Ok(format!("24-bit WAV · {display}"))
        });
        self.show_toast("Export queued in the background");
    }

    pub(crate) fn export_with_preset(&mut self, preset: alp_media::ExportPreset) {
        let scene = match self.session.scene() {
            Ok(scene) => scene,
            Err(error) => {
                self.show_toast(format!("Export failed: {error}"));
                return;
            }
        };
        let Some(dirs) = alp_core::AppDirs::discover().ok() else {
            self.show_toast("No application data directory");
            return;
        };
        let directory = dirs.projects_dir().join("ui-demo").join("renders");
        if let Err(error) = std::fs::create_dir_all(&directory) {
            self.show_toast(format!("Cannot create render folder: {error}"));
            return;
        }
        let settings = preset.settings();
        let path = directory.join(format!(
            "export-{}-{}.{}",
            preset.label(),
            now_ms(),
            settings.format.label()
        ));
        let display = path.display().to_string();
        let master_gain = self.session.master_gain();
        let label = format!("Export {}", preset.label());
        self.session.set_export_queued(&path);
        self.jobs.spawn(label, move |reporter| {
            reporter.progress(0.05);
            reporter.detail("Rendering timeline with preset");
            let mut buffer = alp_media::PcmBuffer::silence(
                scene.sample_rate(),
                scene.channels(),
                scene.total_frames(),
            )
            .map_err(|error| error.to_string())?;
            let mut mixer = alp_engine::Mixer::with_master(scene, master_gain, None, 0.0, 0.0);
            let blocksamples = alp_engine::DEFAULT_BLOCK_FRAMES * usize::from(buffer.channels());
            for block in buffer.samples_mut().chunks_mut(blocksamples) {
                mixer.render_into(block);
            }
            reporter.progress(0.65);
            reporter.detail(format!("Exporting {}", settings.format.label()));
            alp_media::export(&path, &buffer, &settings).map_err(|error| error.to_string())?;
            reporter.progress(1.0);
            Ok(format!(
                "{} · {display}",
                settings.format.label().to_uppercase()
            ))
        });
        self.show_toast(format!("{} queued in the background", preset.label()));
    }
}
