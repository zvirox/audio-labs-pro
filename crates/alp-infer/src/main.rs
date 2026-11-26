//! `alp-infer` — the out-of-process inference worker for Audio Labs Pro.
//!
//! Listens on a Unix domain socket, processes synthesis requests, and isolates
//! model memory and compute from the main application. Engines live for the
//! daemon's lifetime: a loaded neural model is reused across connections, and
//! the builtin tier serves requests before (and after) any model load.

#[cfg(test)]
mod gguf;
mod demucs;
mod kitten;
mod kokoro;
mod moonshine;
mod qwen3;
mod style_tts2;

use std::fs;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use alp_media::PcmBuffer;
use alp_voice::client::BuiltinAcousticBackend;
use alp_voice::protocol::{
    InferRequest, InferResponse, default_socket_path, read_message, write_message,
};
use alp_voice::{
    AcousticCloneBackend, BuiltinSttBackend, CloneBackend, ModelId, ModelManager, SttBackend,
    TranscriptSegment, TtsBackend, TtsRequest, VoiceError, VoiceKind, VoiceProfile,
};

use kitten::KittenEngine;
use kokoro::KokoroEngine;
use moonshine::MoonshineEngine;
use qwen3::Qwen3Engine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut socket_path = default_socket_path();
    let mut run_once = false;

    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--socket" if i + 1 < args.len() => {
                socket_path = PathBuf::from(&args[i + 1]);
                i += 1;
            }
            "--once" => {
                run_once = true;
            }
            "--help" | "-h" => {
                println!("alp-infer — out-of-process inference worker for Audio Labs Pro");
                println!();
                println!("Usage: alp-infer [OPTIONS]");
                println!();
                println!("Options:");
                println!(
                    "  --socket <PATH>   Unix domain socket path (default: $XDG_RUNTIME_DIR/alp-infer.sock)"
                );
                println!("  --once            Handle a single connection and exit");
                println!("  --help            Print help information");
                return Ok(());
            }
            other => {
                eprintln!("unknown argument: {other}");
            }
        }
        i += 1;
    }

    if socket_path.starts_with("/tmp") {
        eprintln!(
            "warning: XDG_RUNTIME_DIR is unset; using a /tmp socket path ({}), which is less isolated",
            socket_path.display()
        );
    }

    if let Some(parent) = socket_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Clear a stale socket from a previous run — but only one that really is
    // a socket of our own user. Anything else is refused, not unlinked.
    prepare_socket_path(&socket_path)?;

    let listener = UnixListener::bind(&socket_path)?;
    harden_socket_permissions(&socket_path)?;
    println!("alp-infer listening on {}", socket_path.display());

    let running = Arc::new(AtomicBool::new(true));

    // One worker for the daemon's lifetime: the builtin tier is constructed
    // once, and a loaded neural model survives across consecutive client
    // connections instead of being reloaded per request.
    let mut worker = Worker::new();

    while running.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _addr)) => {
                let running_clone = running.clone();
                handle_client(stream, &mut worker, running_clone);
                if run_once {
                    break;
                }
            }
            Err(e) => {
                eprintln!("accept failed: {e}");
                break;
            }
        }
    }

    // Cleanup socket on exit
    if socket_path.exists() {
        let _ = fs::remove_file(&socket_path);
    }

    println!("alp-infer shutdown complete");
    Ok(())
}

/// The synthesis engine currently serving requests. The builtin acoustic
/// tier always exists; the neural tier takes over only after a successful
/// `LoadModel`, and a failed load keeps the previously active engine.
enum TtsEngine {
    Builtin(BuiltinAcousticBackend),
    /// A loaded `style_text_to_speech_2` engine (Kokoro-82M, Kitten Nano),
    /// dispatched through the shared [`TtsBackend`] trait (ADR 0023).
    Neural(Box<dyn TtsBackend>),
}

/// The two STT tiers mirror [`TtsEngine`]: the built-in acoustic backend is
/// always available, and a neural STT model (Moonshine Tiny, ADR 0025) takes
/// over only after a successful `LoadModel`.
enum SttEngine {
    Builtin(BuiltinSttBackend),
    /// A loaded neural STT engine dispatched through [`SttBackend`].
    Neural(Box<dyn SttBackend>),
}

/// The clone tier: backends able to render a *cloned* voice from a profile.
///
/// The acoustic tier is always present and needs no model download, so a
/// cloned profile renders on a machine with nothing installed. A neural clone
/// backend (Qwen3-TTS) replaces it in the same slot after a successful load.
enum CloneEngine {
    /// The dependency-free acoustic tier.
    Acoustic(AcousticCloneBackend),
    /// A loaded neural clone backend (Qwen3-TTS).
    Neural(Box<dyn CloneBackend>),
}

impl CloneEngine {
    /// Render a profile with whichever clone backend is active.
    fn synthesize_with(
        &self,
        profile: &VoiceProfile,
        request: &TtsRequest,
    ) -> Result<PcmBuffer, VoiceError> {
        match self {
            CloneEngine::Acoustic(_) if profile.backend == qwen3::MODEL_ID => {
                Err(VoiceError::Unavailable(
                    "Qwen3-TTS is not loaded; refusing to substitute the acoustic clone tier"
                        .into(),
                ))
            }
            CloneEngine::Acoustic(backend) => backend.synthesize_with(profile, request),
            CloneEngine::Neural(backend) => backend.synthesize_with(profile, request),
        }
    }
}

/// Per-daemon state shared by all connections. Clients are handled
/// sequentially on the accept loop, so a loaded model is constructed once
/// and no locking is needed.
///
/// The synthesis (`engine`) and transcription (`stt`) slots are independent:
/// loading one never disturbs the other.
struct Worker {
    engine: TtsEngine,
    stt: SttEngine,
    clone: CloneEngine,
    stem: Option<demucs::DemucsEngine>,
    active_model: Option<String>,
    active_stt: Option<String>,
    active_clone: Option<String>,
    active_stem: Option<String>,
}

impl Worker {
    fn new() -> Self {
        Self {
            engine: TtsEngine::Builtin(BuiltinAcousticBackend::new()),
            stt: SttEngine::Builtin(BuiltinSttBackend::new()),
            clone: CloneEngine::Acoustic(AcousticCloneBackend::new()),
            stem: None,
            active_model: Some("builtin-acoustic".into()),
            active_stt: None,
            active_clone: None,
            active_stem: None,
        }
    }

    /// Status response reflecting the engines actually serving requests.
    fn status(&self) -> InferResponse {
        InferResponse::Status {
            active_model: self.active_model.clone(),
            ready: true,
            active_stt_model: self.active_stt.clone(),
            active_clone_model: self.active_clone.clone(),
            active_stem_model: self.active_stem.clone(),
        }
    }

    /// Status after a declined or failed load: the requested model is not
    /// ready, and the previously active engines keep serving.
    fn declined(&self) -> InferResponse {
        InferResponse::Status {
            active_model: self.active_model.clone(),
            ready: false,
            active_stt_model: self.active_stt.clone(),
            active_clone_model: self.active_clone.clone(),
            active_stem_model: self.active_stem.clone(),
        }
    }

    /// Handle `LoadModel` for the requested model.
    ///
    /// The response's `ready` refers to the *requested* model: `true` only
    /// when it is now the active engine. On failure the previous engine
    /// keeps serving and `ready` is `false`.
    fn load_model(&mut self, model_id: &str) -> InferResponse {
        let dir = if model_id == qwen3::MODEL_ID {
            std::env::var_os("ALP_QWEN3_DIR").map_or_else(
                || ModelManager::new().model_dir(&ModelId::new(model_id)),
                PathBuf::from,
            )
        } else if model_id == demucs::MODEL_ID {
            std::env::var_os("ALP_DEMUCS_DIR").map_or_else(
                || ModelManager::new().model_dir(&ModelId::new(model_id)),
                PathBuf::from,
            )
        } else {
            ModelManager::new().model_dir(&ModelId::new(model_id))
        };
        self.load_model_from(model_id, &dir)
    }

    /// Load a model from an explicit directory (injected for tests).
    ///
    /// Model ids dispatch by tier: STT models (`moonshine-tiny`) load into
    /// the transcription slot without touching synthesis; clone models
    /// (`qwen3-tts`) load into the clone slot without touching synthesis
    /// either; everything else loads into the synthesis slot.
    /// `builtin-acoustic` resets synthesis only — the built-in STT backend
    /// never unloads, it is simply shadowed while a neural STT model is
    /// loaded, and the clone tier is likewise left alone.
    fn load_model_from(&mut self, model_id: &str, model_dir: &Path) -> InferResponse {
        if model_id == "builtin-acoustic" || model_id.is_empty() {
            self.engine = TtsEngine::Builtin(BuiltinAcousticBackend::new());
            self.active_model = Some("builtin-acoustic".into());
            return self.status();
        }

        // Neural STT slot first (ADR 0025): independent of the TTS slot.
        if model_id == moonshine::MODEL_ID {
            let loaded = MoonshineEngine::load(model_dir)
                .map(|engine| Box::new(engine) as Box<dyn SttBackend>);
            return match loaded {
                Ok(engine) => {
                    self.stt = SttEngine::Neural(engine);
                    self.active_stt = Some(model_id.to_owned());
                    eprintln!("worker: {model_id} loaded; neural transcription active");
                    self.status()
                }
                Err(error) => {
                    eprintln!(
                        "worker: loading {model_id} failed, keeping {}: {error}",
                        self.active_stt.as_deref().unwrap_or("builtin STT")
                    );
                    self.declined()
                }
            };
        }

        // Clone slot: a heavy clone backend replaces the acoustic tier without
        // disturbing synthesis or transcription.
        if model_id == qwen3::MODEL_ID {
            let loaded = Qwen3Engine::load(model_dir);
            return match loaded {
                Ok(engine) => {
                    eprintln!(
                        "worker: {model_id} loaded from {} (talker {}, decoder {}; {} Hz output); neural cloning active",
                        engine.model_dir().display(),
                        engine.talker_path().display(),
                        engine.decoder_path().display(),
                        qwen3::SAMPLE_RATE
                    );
                    self.clone = CloneEngine::Neural(Box::new(engine));
                    self.active_clone = Some(model_id.to_owned());
                    self.status()
                }
                Err(error) => {
                    eprintln!(
                        "worker: loading {model_id} failed, keeping {}: {error}",
                        self.active_clone
                            .as_deref()
                            .unwrap_or("the acoustic clone tier")
                    );
                    self.declined()
                }
            };
        }

        // Stem separation slot: a Demucs-style ONNX export loads independently
        // of TTS/STT/clone tiers and only serves `SeparateStems`.
        if model_id == demucs::MODEL_ID {
            let loaded = match demucs::DemucsEngine::from_environment() {
                Some(result) => result,
                None => demucs::DemucsEngine::load(model_dir),
            };
            return match loaded {
                Ok(engine) => {
                    eprintln!(
                        "worker: {model_id} loaded; four-stem separation active ({} Hz, {} channel input)",
                        engine.manifest().sample_rate,
                        if engine.manifest().stereo { 2 } else { 1 }
                    );
                    self.stem = Some(engine);
                    self.active_stem = Some(model_id.to_owned());
                    self.status()
                }
                Err(error) => {
                    eprintln!("worker: loading {model_id} failed: {error}");
                    self.declined()
                }
            };
        }

        // Each wired neural model maps to one engine constructor; anything
        // else is declined honestly and the active engine keeps serving.
        let loaded = match model_id {
            kokoro::MODEL_ID => {
                KokoroEngine::load(model_dir).map(|engine| Box::new(engine) as Box<dyn TtsBackend>)
            }
            kitten::MODEL_ID => {
                KittenEngine::load(model_dir).map(|engine| Box::new(engine) as Box<dyn TtsBackend>)
            }
            _ => {
                eprintln!("worker: LoadModel {model_id}: no engine is wired for this model yet");
                return self.declined();
            }
        };

        match loaded {
            Ok(engine) => {
                self.engine = TtsEngine::Neural(engine);
                self.active_model = Some(model_id.to_owned());
                eprintln!("worker: {model_id} loaded; neural synthesis active");
                self.status()
            }
            Err(error) => {
                eprintln!(
                    "worker: loading {model_id} failed, keeping {}: {error}",
                    self.active_model.as_deref().unwrap_or("no engine")
                );
                self.declined()
            }
        }
    }

    /// Synthesize with whichever engine is active.
    fn synthesize(&self, request: &TtsRequest) -> Result<PcmBuffer, VoiceError> {
        match &self.engine {
            TtsEngine::Builtin(backend) => backend.synthesize(request),
            TtsEngine::Neural(engine) => engine.synthesize(request),
        }
    }

    /// Transcribe with whichever STT engine is active.
    fn transcribe(&self, pcm: &PcmBuffer) -> Result<Vec<TranscriptSegment>, VoiceError> {
        match &self.stt {
            SttEngine::Builtin(backend) => backend.transcribe(pcm),
            SttEngine::Neural(engine) => engine.transcribe(pcm),
        }
    }
}

fn handle_client(mut stream: UnixStream, worker: &mut Worker, running: Arc<AtomicBool>) {
    // The socket is created 0600, so the kernel refuses connect() from any
    // other user before this loop is ever reached. A per-peer credential
    // check on accept would additionally need an unsafe getsockopt call,
    // which the workspace `unsafe_code = deny` lint excludes.
    loop {
        let request: Result<InferRequest, _> = read_message(&mut stream);
        let req = match request {
            Ok(r) => r,
            Err(_) => break, // Connection closed by client
        };

        match req {
            InferRequest::Ping => {
                if write_message(&mut stream, &InferResponse::Pong).is_err() {
                    break;
                }
            }
            InferRequest::GetStatus => {
                let resp = worker.status();
                if write_message(&mut stream, &resp).is_err() {
                    break;
                }
            }
            InferRequest::LoadModel { model_id } => {
                let resp = worker.load_model(&model_id);
                if write_message(&mut stream, &resp).is_err() {
                    break;
                }
            }
            InferRequest::Synthesize { id, request } => {
                let _ = write_message(&mut stream, &InferResponse::Progress { id, progress: 0.2 });

                match worker.synthesize(&request) {
                    Ok(pcm) => {
                        let _ = write_message(
                            &mut stream,
                            &InferResponse::Progress { id, progress: 1.0 },
                        );
                        let resp = InferResponse::Audio {
                            id,
                            sample_rate: pcm.sample_rate(),
                            channels: pcm.channels(),
                            samples: pcm.samples().to_vec(),
                        };
                        if write_message(&mut stream, &resp).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let resp = InferResponse::Error {
                            id,
                            message: e.to_string(),
                        };
                        if write_message(&mut stream, &resp).is_err() {
                            break;
                        }
                    }
                }
            }
            InferRequest::SynthesizeWithProfile {
                id,
                profile,
                request,
            } => {
                let _ = write_message(&mut stream, &InferResponse::Progress { id, progress: 0.2 });

                // The human-only consent rule is enforced here, at the only
                // place a clone can actually be rendered. A profile of kind
                // `Cloned` without an attested consent record is refused.
                let refusal = match profile.kind {
                    VoiceKind::Cloned if profile.consent.is_none() => {
                        Some("refusing to render a cloned voice without a human consent record")
                    }
                    _ => None,
                };

                let outcome = match refusal {
                    Some(message) => Err(VoiceError::Backend(message.to_owned())),
                    None => worker.clone.synthesize_with(&profile, &request),
                };

                match outcome {
                    Ok(pcm) => {
                        let _ = write_message(
                            &mut stream,
                            &InferResponse::Progress { id, progress: 1.0 },
                        );
                        let resp = InferResponse::Audio {
                            id,
                            sample_rate: pcm.sample_rate(),
                            channels: pcm.channels(),
                            samples: pcm.samples().to_vec(),
                        };
                        if write_message(&mut stream, &resp).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let resp = InferResponse::Error {
                            id,
                            message: e.to_string(),
                        };
                        if write_message(&mut stream, &resp).is_err() {
                            break;
                        }
                    }
                }
            }
            InferRequest::Transcribe {
                id,
                sample_rate,
                channels,
                samples,
            } => {
                let _ = write_message(&mut stream, &InferResponse::Progress { id, progress: 0.2 });
                match PcmBuffer::new(sample_rate, channels, samples) {
                    Ok(pcm) => match worker.transcribe(&pcm) {
                        Ok(segments) => {
                            let _ = write_message(
                                &mut stream,
                                &InferResponse::Progress { id, progress: 1.0 },
                            );
                            let resp = InferResponse::Transcript { id, segments };
                            if write_message(&mut stream, &resp).is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            let resp = InferResponse::Error {
                                id,
                                message: e.to_string(),
                            };
                            if write_message(&mut stream, &resp).is_err() {
                                break;
                            }
                        }
                    },
                    Err(e) => {
                        let resp = InferResponse::Error {
                            id,
                            message: e.to_string(),
                        };
                        if write_message(&mut stream, &resp).is_err() {
                            break;
                        }
                    }
                }
            }
            InferRequest::DesignVoice { id, request } => {
                let profile = alp_voice::design_voice(&request);
                let resp = InferResponse::VoiceDesigned {
                    id,
                    profile: Box::new(profile),
                };
                if write_message(&mut stream, &resp).is_err() {
                    break;
                }
            }
            InferRequest::SeparateStems {
                id,
                sample_rate,
                channels,
                samples,
            } => {
                let _ =
                    write_message(&mut stream, &InferResponse::Progress { id, progress: 0.2 });
                let outcome = (|| -> Result<[PcmBuffer; 4], VoiceError> {
                    let engine = worker.stem.as_ref().ok_or_else(|| {
                        VoiceError::Unavailable(
                            "demucs-4stem model is not loaded; call LoadModel first".into(),
                        )
                    })?;
                    let pcm = PcmBuffer::new(sample_rate, channels, samples)
                        .map_err(|error| VoiceError::Backend(error.to_string()))?;
                    engine.separate(&pcm)
                })();
                match outcome {
                    Ok(stems) => {
                        let _ = write_message(
                            &mut stream,
                            &InferResponse::Progress { id, progress: 1.0 },
                        );
                        let resp = InferResponse::Stems {
                            id,
                            sample_rate: stems[0].sample_rate(),
                            channels: stems[0].channels(),
                            stems: stems.iter().map(|stem| stem.samples().to_vec()).collect(),
                            names: demucs::STEM_NAMES.iter().map(|name| (*name).to_owned()).collect(),
                        };
                        if write_message(&mut stream, &resp).is_err() {
                            break;
                        }
                    }
                    Err(error) => {
                        let resp = InferResponse::Error {
                            id,
                            message: error.to_string(),
                        };
                        if write_message(&mut stream, &resp).is_err() {
                            break;
                        }
                    }
                }
            }
            InferRequest::Cancel { id } => {
                let _ = write_message(
                    &mut stream,
                    &InferResponse::Error {
                        id,
                        message: "request cancelled".into(),
                    },
                );
            }
            InferRequest::Shutdown => {
                running.store(false, Ordering::SeqCst);
                let _ = write_message(&mut stream, &InferResponse::Pong);
                break;
            }
        }
    }
}

/// Whether a stale path is safe to clear: it must be a Unix socket owned by
/// the uid passed in. `uid` of `None` (owner could not be established) fails
/// closed, and a `Symlink` file type is never a socket, so links are refused.
fn may_remove_stale_socket(meta: &fs::Metadata, uid: Option<u32>) -> bool {
    match uid {
        Some(uid) => meta.file_type().is_socket() && meta.uid() == uid,
        None => false,
    }
}

/// Clear the socket path if it holds a stale socket of our own user from a
/// previous run. Anything else — a regular file, a symlink, another user's
/// socket, or an owner we cannot establish — is refused, never unlinked.
fn prepare_socket_path(socket_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(meta) = fs::symlink_metadata(socket_path) {
        let dir = socket_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty());
        let own = dir.and_then(socket_owner_uid);
        if !may_remove_stale_socket(&meta, own) {
            return Err(format!(
                "refusing to remove {}: not a stale socket owned by this user",
                socket_path.display()
            )
            .into());
        }
        fs::remove_file(socket_path)?;
    }
    Ok(())
}

/// Restrict a freshly bound listener socket to its owner. Without this the
/// socket inherits the umask, and other users could connect, flood the
/// transcription endpoint, or shut the worker down.
fn harden_socket_permissions(socket_path: &Path) -> std::io::Result<()> {
    fs::set_permissions(socket_path, fs::Permissions::from_mode(0o600))
}

/// The uid this process's bound sockets receive, learned by binding a
/// throwaway probe socket and reading its owner. Safe std only (the
/// workspace denies `unsafe_code`, so there is no `geteuid`); returns `None`
/// when the probe cannot be created, which makes callers fail closed.
fn socket_owner_uid(dir: &Path) -> Option<u32> {
    let probe = dir.join(".alp-infer-uid-probe");
    let listener = UnixListener::bind(&probe).ok()?;
    let uid = fs::symlink_metadata(&probe).ok()?.uid();
    drop(listener);
    let _ = fs::remove_file(&probe);
    Some(uid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepare_socket_path_refuses_a_regular_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let victim = dir.path().join("alchemy.sock");
        fs::write(&victim, b"not a socket").expect("file");

        assert!(
            prepare_socket_path(&victim).is_err(),
            "a regular file must never be silently unlinked"
        );
        assert!(victim.exists(), "the refused file must stay untouched");
    }

    #[test]
    fn prepare_socket_path_clears_a_stale_owned_socket() {
        let dir = tempfile::tempdir().expect("tempdir");
        let stale = dir.path().join("alp-infer.sock");
        let listener = UnixListener::bind(&stale).expect("bind");
        drop(listener); // leaves the stale socket file behind

        prepare_socket_path(&stale).expect("clear stale socket");
        assert!(!stale.exists());
    }

    #[test]
    fn socket_permissions_are_hardened_to_owner_only() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("alp-infer.sock");
        let listener = UnixListener::bind(&path).expect("bind");
        harden_socket_permissions(&path).expect("harden");

        let mode = fs::metadata(&path).expect("meta").permissions().mode();
        assert_eq!(
            mode & 0o777,
            0o600,
            "the inference socket must be owner-only, got {:o}",
            mode & 0o777
        );
        drop(listener);
    }

    #[test]
    fn may_remove_stale_socket_decides_by_type_and_owner() {
        let dir = tempfile::tempdir().expect("tempdir");
        let regular = dir.path().join("regular.file");
        fs::write(&regular, b"keep me").expect("file");
        let socket = dir.path().join("stale.sock");
        let listener = UnixListener::bind(&socket).expect("bind");

        let own = socket_owner_uid(dir.path());
        let regular_meta = fs::symlink_metadata(&regular).expect("meta");
        let socket_meta = fs::symlink_metadata(&socket).expect("meta");

        assert!(
            !may_remove_stale_socket(&regular_meta, own),
            "regular files must never be clearable"
        );
        assert!(
            may_remove_stale_socket(&socket_meta, own),
            "our own stale socket must be clearable"
        );
        if let Some(uid) = own {
            // A socket owned by somebody else must not be removed either.
            let other = if uid == u32::MAX { uid - 1 } else { uid + 1 };
            assert!(!may_remove_stale_socket(&socket_meta, Some(other)));
        }
        assert!(
            !may_remove_stale_socket(&socket_meta, None),
            "when the owner uid cannot be established, fail closed"
        );
        drop(listener);
    }

    #[test]
    fn sockets_we_bind_are_reported_as_our_own() {
        let dir = tempfile::tempdir().expect("tempdir");
        let socket = dir.path().join("probe.sock");
        let listener = UnixListener::bind(&socket).expect("bind");

        let own = socket_owner_uid(dir.path()).expect("own uid");
        let meta = fs::symlink_metadata(&socket).expect("meta");
        assert!(
            meta.uid() == own,
            "the probe must report the uid of sockets we bind"
        );
        drop(listener);
    }

    #[test]
    fn a_fresh_worker_reports_the_builtin_engine_as_ready() {
        let worker = Worker::new();
        assert_eq!(
            worker.status(),
            InferResponse::Status {
                active_model: Some("builtin-acoustic".into()),
                ready: true,
                active_stt_model: None,
                active_clone_model: None,
                active_stem_model: None,
            }
        );
    }

    #[test]
    fn a_failed_model_load_keeps_the_builtin_engine() {
        let mut worker = Worker::new();

        // The load must fail honestly when the model directory is absent.
        assert_eq!(
            worker.load_model_from("kokoro-82m", Path::new("/nonexistent/alp-models")),
            InferResponse::Status {
                active_model: Some("builtin-acoustic".into()),
                ready: false,
                active_stt_model: None,
                active_clone_model: None,
                active_stem_model: None,
            }
        );

        // The previously active engine keeps synthesizing after the failure.
        let request = TtsRequest {
            text: "Test line".into(),
            voice_id: "aura".into(),
            speed: 1.0,
        };
        assert!(worker.synthesize(&request).is_ok());

        // Unknown identifiers never leave the current engine.
        assert_eq!(
            worker.load_model("zz-not-a-model"),
            InferResponse::Status {
                active_model: Some("builtin-acoustic".into()),
                ready: false,
                active_stt_model: None,
                active_clone_model: None,
                active_stem_model: None,
            }
        );

        // Switching back to the builtin tier always succeeds.
        assert_eq!(
            worker.load_model("builtin-acoustic"),
            InferResponse::Status {
                active_model: Some("builtin-acoustic".into()),
                ready: true,
                active_stt_model: None,
                active_clone_model: None,
                active_stem_model: None,
            }
        );
    }

    #[test]
    fn a_kitten_load_without_files_declines_and_keeps_serving() {
        let mut worker = Worker::new();

        // The wired kitten model fails honestly when its directory is absent
        // (no downloaded weights), and the builtin tier keeps serving.
        assert_eq!(
            worker.load_model_from("kitten-nano", Path::new("/nonexistent/alp-models")),
            InferResponse::Status {
                active_model: Some("builtin-acoustic".into()),
                ready: false,
                active_stt_model: None,
                active_clone_model: None,
                active_stem_model: None,
            }
        );
        let request = TtsRequest {
            text: "Still here".into(),
            voice_id: "aura".into(),
            speed: 1.0,
        };
        assert!(worker.synthesize(&request).is_ok());
    }

    #[test]
    fn a_moonshine_decline_keeps_builtin_stt_and_the_status_reports_it() {
        let mut worker = Worker::new();

        // Moonshine is a wired STT model, so a missing directory fails
        // honestly at its tier: ready is false and the built-in STT keeps
        // transcribing. The synthesis slot is untouched by the attempt.
        assert_eq!(
            worker.load_model_from("moonshine-tiny", Path::new("/nonexistent/alp-models")),
            InferResponse::Status {
                active_model: Some("builtin-acoustic".into()),
                ready: false,
                active_stt_model: None,
                active_clone_model: None,
                active_stem_model: None,
            }
        );

        // The built-in STT backend still answers through the dispatch helper.
        let silence = PcmBuffer::new(16_000, 1, Vec::new()).expect("pcm");
        assert!(worker.transcribe(&silence).is_ok());

        // An unregistered identifier is declined for both tiers alike.
        assert_eq!(
            worker.load_model("zz-not-a-model"),
            InferResponse::Status {
                active_model: Some("builtin-acoustic".into()),
                ready: false,
                active_stt_model: None,
                active_clone_model: None,
                active_stem_model: None,
            }
        );
    }

    #[test]
    fn a_corrupt_kitten_graph_declines_and_keeps_serving() {
        // Fixture models cannot provide a real ONNX graph in-process here
        // (weights are binary artifacts, not derivable), so this covers only
        // the dispatch contract: the kitten branch is reached and reports
        // ready:false with its honest reason when the graph is invalid, and
        // the previous engine survives. The real activation path is pinned
        // by the kitten smoke test and the socket-level daemon tests.
        let dir = tempfile::tempdir().expect("tempdir");
        let mut worker = Worker::new();

        // A tokenizer-only directory satisfies the file presence checks but
        // fails at the vocabulary stage; either way the load is declined
        // rather than silently accepted.
        std::fs::create_dir(dir.path().join("onnx")).expect("fixture dirs");
        std::fs::create_dir(dir.path().join("voices")).expect("fixture dirs");
        std::fs::write(dir.path().join("onnx/model_quantized.onnx"), b"not a graph")
            .expect("fixture");
        std::fs::write(dir.path().join("tokenizer.json"), b"{}").expect("fixture");

        assert_eq!(
            worker.load_model_from("kitten-nano", dir.path()),
            InferResponse::Status {
                active_model: Some("builtin-acoustic".into()),
                ready: false,
                active_stt_model: None,
                active_clone_model: None,
                active_stem_model: None,
            }
        );
    }

    /// With both real models installed, the two slots must load and serve
    /// independently: loading Moonshine never unloads a loaded synthesis
    /// model, and the `builtin-acoustic` reset touches synthesis only.
    /// Skipped when either model is absent (standard XDG location or
    /// `ALP_MOONSHINE_DIR` / `ALP_KITTEN_DIR`).
    #[test]
    fn two_engine_slots_load_and_reset_independently_when_installed() {
        let storage = alp_voice::ModelManagerConfig::default().storage_dir;
        let moonshine_dir = std::env::var("ALP_MOONSHINE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| storage.join("moonshine-tiny"));
        let kitten_dir = std::env::var("ALP_KITTEN_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| storage.join("kitten-nano"));
        if !moonshine_dir.join(moonshine::ENCODER_FILE).is_file()
            || !kitten_dir.join("onnx/model_quantized.onnx").is_file()
        {
            eprintln!("skip: moonshine-tiny / kitten-nano not both installed");
            return;
        }

        let mut worker = Worker::new();

        // Slot 1: the synthesis tier takes kitten.
        assert_eq!(
            worker.load_model_from("kitten-nano", &kitten_dir),
            InferResponse::Status {
                active_model: Some("kitten-nano".into()),
                ready: true,
                active_stt_model: None,
                active_clone_model: None,
                active_stem_model: None,
            }
        );

        // Slot 2: loading the STT model leaves synthesis untouched.
        assert_eq!(
            worker.load_model_from("moonshine-tiny", &moonshine_dir),
            InferResponse::Status {
                active_model: Some("kitten-nano".into()),
                ready: true,
                active_stt_model: Some("moonshine-tiny".into()),
                active_clone_model: None,
                active_stem_model: None,
            }
        );

        // Both tiers serve through the dispatch helpers.
        let spoken = worker
            .synthesize(&TtsRequest {
                text: "Hello there".into(),
                voice_id: "expr-voice-2-f".into(),
                speed: 1.0,
            })
            .expect("kitten synthesis alongside STT");
        let segments = worker.transcribe(&spoken).expect("moonshine transcription");
        assert!(
            segments.iter().all(|s| !s.text.trim().is_empty()),
            "the neural STT tier produced text: {segments:?}"
        );

        // The reset id restores the builtin synthesis tier and leaves the
        // transcription slot alone.
        assert_eq!(
            worker.load_model("builtin-acoustic"),
            InferResponse::Status {
                active_model: Some("builtin-acoustic".into()),
                ready: true,
                active_stt_model: Some("moonshine-tiny".into()),
                active_clone_model: None,
                active_stem_model: None,
            }
        );
    }

    #[test]
    fn a_qwen3_load_without_files_declines_and_keeps_serving() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut worker = Worker::new();
        assert_eq!(
            worker.load_model_from("qwen3-tts", dir.path()),
            InferResponse::Status {
                active_model: Some("builtin-acoustic".into()),
                ready: false,
                active_stt_model: None,
                active_clone_model: None,
                active_stem_model: None,
            }
        );

        let mut neural_profile = VoiceProfile::builtin("unloaded-qwen", "Unloaded Qwen", vec![]);
        neural_profile.kind = VoiceKind::Cloned;
        neural_profile.backend = qwen3::MODEL_ID.into();
        let error = worker
            .clone
            .synthesize_with(
                &neural_profile,
                &TtsRequest {
                    text: "Must not silently become acoustic".into(),
                    voice_id: neural_profile.id.clone(),
                    speed: 1.0,
                },
            )
            .expect_err("a Qwen profile cannot use the acoustic slot");
        assert!(
            error.to_string().contains("refusing to substitute"),
            "{error}"
        );

        let pcm = worker
            .synthesize(&TtsRequest {
                text: "Still serving via builtin".into(),
                voice_id: "aura".into(),
                speed: 1.0,
            })
            .expect("builtin synthesis survives decline");
        assert!(pcm.duration_secs() > 0.0);
    }
}
