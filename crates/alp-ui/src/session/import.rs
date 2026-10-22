//! Background import worker for the editing session.

use super::*;

pub(super) struct ImportedAudio {
    pub(super) buffer: PcmBuffer,
    pub(super) name: String,
    pub(super) path: String,
}

impl EditSession {
    /// Whether a background import is still running.
    #[must_use]
    pub fn is_importing(&self) -> bool {
        self.import_rx.is_some()
    }

    /// Start decoding a dropped file on a worker thread.
    pub fn begin_import(&mut self, path: PathBuf) -> Result<(), String> {
        if self.import_rx.is_some() {
            return Err("An import is already running".into());
        }
        let display = path.to_string_lossy().into_owned();
        let name = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Imported audio".into());
        let (sender, receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("alp-ui-import".into())
            .spawn(move || {
                let result = decode(&path)
                    .map(|buffer| ImportedAudio {
                        buffer,
                        name,
                        path: display,
                    })
                    .map_err(|error| error.to_string());
                let _ = sender.send(result);
            })
            .map_err(|error| format!("Cannot start import: {error}"))?;
        self.import_rx = Some(receiver);
        self.status = "Decoding import in the background".into();
        Ok(())
    }

    /// Poll a background import. Returns true when a file was added.
    pub fn poll_import(&mut self) -> bool {
        let Some(receiver) = &self.import_rx else {
            return false;
        };
        match receiver.try_recv() {
            Ok(Ok(audio)) => {
                self.import_rx = None;
                self.import_buffer(audio.buffer, &audio.name, &audio.path);
                true
            }
            Ok(Err(error)) => {
                self.import_rx = None;
                self.status = format!("Import failed: {error}");
                false
            }
            Err(TryRecvError::Empty) => false,
            Err(TryRecvError::Disconnected) => {
                self.import_rx = None;
                self.status = "Import worker stopped unexpectedly".into();
                false
            }
        }
    }
}
