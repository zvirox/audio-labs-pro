//! `alp-infer` — the out-of-process inference worker for Audio Labs Pro.
//!
//! Listens on a Unix domain socket, processes synthesis requests, and isolates
//! model memory and compute from the main application.

use std::fs;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
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

    if let Some(parent) = socket_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Clean up stale socket file if it exists
    if socket_path.exists() {
        let _ = fs::remove_file(&socket_path);
    }

    let listener = UnixListener::bind(&socket_path)?;
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
