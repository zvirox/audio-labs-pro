//! Neural-tier actions for the UI: resumable downloads, engine checks and
//! model loading.
//!
//! All blocking work (HTTPS transfer, socket I/O) runs on the shared
//! [`JobQueue`](crate::jobs::JobQueue) worker threads, so the render loop never
//! blocks. Results flow back through the job channel and the engine probe slot,
//! drained on every `Message::Frame`.

use std::path::PathBuf;
use std::sync::Arc;

use alp_media::PcmBuffer;
use alp_voice::{
    BuiltinAcousticBackend, DownloadError, InferClient, ModelDownloader, ModelId, ModelManager,
    ModelManagerConfig, TtsBackend as _, TtsRequest, VoiceError,
};

use super::App;

/// Observed state of the `alp-infer` daemon.
///
/// Refreshed on demand (`Check engine` button, after downloads complete, after
/// load attempts) and never polled in the background, so the UI never performs
/// hidden I/O the user did not ask for.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct EngineState {
    /// The daemon answered the last probe.
    pub alive: bool,
    /// Model id of the engine currently active in the daemon.
    pub active_model: Option<String>,
    /// Whether the active model reports itself ready.
    pub ready: bool,
}

impl EngineState {
    /// One-line summary for chips and status rows.
    pub(crate) fn label(&self) -> String {
        if !self.alive {
            return "daemon not detected".to_owned();
        }
        match (&self.active_model, self.ready) {
            (Some(model), true) => format!("ready - active model {model}"),
            (Some(model), false) => format!("loaded - {model} not ready"),
            (None, _) => "alive - builtin tier".to_owned(),
        }
    }
}

impl App {
    /// Start a resumable background download for a model.
    ///
    /// Gates are enforced a second time inside the worker: a fresh
    /// `ModelManager` re-reads the persisted license markers, so the button and
    /// the transfer can never disagree about state.
    pub(super) fn download_model(&mut self, id: &str) {
        let Some(manifest) = self.model_manager.get_manifest(&ModelId::new(id)) else {
            return;
        };
        if manifest.is_builtin {
            self.show_toast(format!("{id} is built in; there is nothing to download"));
            return;
        }
        if !manifest.license.accepted {
            self.show_toast("Accept the license first; the download gate enforces it too");
            return;
        }
        if self.model_jobs.contains_key(id) {
            return; // Already running; the panel shows live progress.
        }
        self.model_errors.remove(id);

        let storage: PathBuf = self.model_manager.storage_dir().to_path_buf();
        let offline = self.models_offline;
        let model_id = id.to_string();
        let display_name = manifest.name.clone();
        let job_id = self
            .jobs
            .spawn(format!("Download {display_name}"), move |reporter| {
                let manager = ModelManager::with_config(ModelManagerConfig {
                    storage_dir: storage,
                    offline_mode: offline,
                });
                let downloader = ModelDownloader::new();
                let id = ModelId::new(model_id.clone());
                let mut last_file = String::new();
                let outcome = downloader.download(&manager, &id, &mut |progress| {
                    let ratio = if progress.overall_total == 0 {
                        1.0
                    } else {
                        progress.overall_done as f32 / progress.overall_total as f32
                    };
                    reporter.progress(ratio);
                    if progress.file_name != last_file {
                        reporter.detail(progress.file_name.clone());
                        last_file = progress.file_name;
                    }
                });
                match outcome {
                    Ok(report) => Ok(format!(
                        "{} file(s) ready ({} fetched, {} reused)",
                        report.files_fetched.len() + report.files_skipped.len(),
                        report.files_fetched.len(),
                        report.files_skipped.len()
                    )),
                    Err(DownloadError::ShortStream {
                        expected, received, ..
                    }) => Err(format!(
                        "stream ended early at {received} of {expected} bytes; retry to resume"
                    )),
                    Err(error) => Err(error.to_string()),
                }
            });
        self.model_jobs.insert(id.to_owned(), job_id);
        self.show_toast(format!(
            "Downloading {display_name}; progress shows on the model row"
        ));
    }

    /// Drain finished download jobs into inline per-model result state.
    pub(super) fn poll_model_jobs(&mut self) {
        let running = self.jobs.jobs().iter().any(|job| job.status.is_running());
        if !running && self.model_jobs.is_empty() {
            return;
        }
        let finished: Vec<(String, u64)> = self
            .model_jobs
            .iter()
            .filter_map(|(model, job_id)| {
                let job = self.jobs.jobs().iter().find(|job| job.id == *job_id)?;
                (!job.status.is_running()).then(|| (model.clone(), *job_id))
            })
            .collect();
        for (model, job_id) in finished {
            self.model_jobs.remove(&model);
            let Some(job) = self.jobs.jobs().iter().find(|job| job.id == job_id) else {
                continue;
            };
            match job.status {
                crate::jobs::JobStatus::Completed => {
                    self.model_errors.remove(&model);
                    self.show_toast(format!("{model}: {}", job.detail));
                    // A fresh install changes what the daemon could load; a
                    // probe makes the next action obvious without guessing.
                    self.check_engine();
                }
                crate::jobs::JobStatus::Failed => {
                    self.model_errors.insert(model, job.detail.clone());
                }
                _ => {}
            }
        }
    }

    /// Probe the daemon once and record the active engine.
    pub(super) fn check_engine(&mut self) {
        if self.engine_checking {
            return;
        }
        let probe = Arc::clone(&self.engine_probe);
        self.engine_checking = true;
        self.jobs.spawn("Engine check", move |reporter| {
            reporter.detail("Pinging the alp-infer daemon");
            let client = InferClient::new();
            let state = match client.status() {
                Ok((active_model, ready)) => EngineState {
                    alive: true,
                    active_model,
                    ready,
                },
                Err(_) => EngineState::default(),
            };
            if let Ok(mut guard) = probe.lock() {
                *guard = Some(state);
            }
            Ok("engine state checked".into())
        });
    }

    /// Ask the daemon to make `id` the active engine.
    ///
    /// The daemon replies with the resulting status; a declined load keeps its
    /// previous engine, and the response states that honestly.
    pub(super) fn load_neural_model(&mut self, id: &str) {
        if !self.engine.alive {
            self.show_toast("No alp-infer daemon is running; press Check engine or start one");
            return;
        }
        let model_id = id.to_string();
        let probe = Arc::clone(&self.engine_probe);
        self.jobs
            .spawn(format!("Load {model_id}"), move |reporter| {
                reporter.detail(format!("Sending LoadModel for {model_id}"));
                let client = InferClient::new();
                let state = match client.load_model(&model_id) {
                    Ok((active_model, ready)) => EngineState {
                        alive: true,
                        active_model,
                        ready,
                    },
                    Err(error) => return Err(format!("the daemon refused the load: {error}")),
                };
                let now_ready = state.ready;
                if let Ok(mut guard) = probe.lock() {
                    *guard = Some(state);
                }
                Ok(if now_ready {
                    format!("{model_id} is now the active engine")
                } else {
                    format!("{model_id} was declined; the previous engine keeps serving")
                })
            });
    }

    /// Consume engine probe results produced by worker threads.
    pub(super) fn drain_engine_probe(&mut self) {
        if let Ok(mut guard) = self.engine_probe.lock() {
            if let Some(state) = guard.take() {
                self.engine = state;
            }
            self.engine_checking = false;
        }
    }
}

/// Synthesis for generation surfaces: prefer the daemon (which serves whichever
/// engine is active - neural when a model is loaded, its builtin fallback
/// otherwise) and fall back to the in-process acoustic engine when the daemon
/// is absent, so a running install never degrades the editor.
///
/// `detail` receives the engine decision so jobs can surface what actually
/// rendered the audio.
pub(crate) fn synthesize_via_tier(
    request: &TtsRequest,
    detail: &mut dyn FnMut(String),
) -> Result<PcmBuffer, VoiceError> {
    let client = InferClient::new();
    if client.is_alive() {
        detail("Synthesizing via the alp-infer engine".to_owned());
        match client.synthesize(request) {
            Ok(buffer) => return Ok(buffer),
            Err(error) => detail(format!(
                "daemon synthesis failed ({error}); using the built-in engine"
            )),
        }
    }
    detail("Synthesizing with the built-in acoustic engine".to_owned());
    BuiltinAcousticBackend::new().synthesize(request)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_label_tracks_daemon_and_model_state() {
        assert_eq!(EngineState::default().label(), "daemon not detected");
        let alive_builtin = EngineState {
            alive: true,
            active_model: None,
            ready: false,
        };
        assert_eq!(alive_builtin.label(), "alive - builtin tier");
        let warming = EngineState {
            alive: true,
            active_model: Some("kokoro-82m".into()),
            ready: false,
        };
        assert_eq!(warming.label(), "loaded - kokoro-82m not ready");
        let ready = EngineState {
            alive: true,
            active_model: Some("kokoro-82m".into()),
            ready: true,
        };
        assert_eq!(ready.label(), "ready - active model kokoro-82m");
    }

    #[test]
    fn synthesis_falls_back_when_no_daemon_listens() {
        // No alp-infer daemon owns the default socket path in the test run, so
        // the tier router must fall back to the built-in engine and keep the
        // surface informed through the detail callback.
        let request = TtsRequest {
            text: "Good morning".into(),
            voice_id: "aura".into(),
            speed: 1.0,
        };
        let mut details = Vec::new();
        let buffer = synthesize_via_tier(&request, &mut |line| details.push(line))
            .expect("built-in synthesis");
        assert!(!buffer.samples().is_empty());
        assert_eq!(buffer.channels(), 1);
        assert!(
            details
                .iter()
                .all(|line| line.contains("engine") || line.contains("built-in"))
        );
    }
}
