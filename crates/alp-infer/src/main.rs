//! `alp-infer` — the out-of-process inference worker for Audio Labs Pro.
//!
//! Listens on a Unix domain socket, processes synthesis requests, and isolates
//! model memory and compute from the main application.

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
use alp_voice::{BuiltinSttBackend, SttBackend, TtsBackend};

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

    while running.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _addr)) => {
                let running_clone = running.clone();
                handle_client(stream, running_clone);
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

fn handle_client(mut stream: UnixStream, running: Arc<AtomicBool>) {
    // The socket is created 0600, so the kernel refuses connect() from any
    // other user before this loop is ever reached. A per-peer credential
    // check on accept would additionally need an unsafe getsockopt call,
    // which the workspace `unsafe_code = deny` lint excludes.
    let mut active_model: Option<String> = Some("builtin-acoustic".into());
    let backend = BuiltinAcousticBackend::new();

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
                let resp = InferResponse::Status {
                    active_model: active_model.clone(),
                    ready: true,
                };
                if write_message(&mut stream, &resp).is_err() {
                    break;
                }
            }
            InferRequest::LoadModel { model_id } => {
                active_model = Some(model_id);
                let resp = InferResponse::Status {
                    active_model: active_model.clone(),
                    ready: true,
                };
                if write_message(&mut stream, &resp).is_err() {
                    break;
                }
            }
            InferRequest::Synthesize { id, request } => {
                let _ = write_message(&mut stream, &InferResponse::Progress { id, progress: 0.2 });

                match backend.synthesize(&request) {
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
                    Ok(pcm) => {
                        let stt_backend = BuiltinSttBackend::new();
                        match stt_backend.transcribe(&pcm) {
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
}
