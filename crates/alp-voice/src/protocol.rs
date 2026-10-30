//! Unix domain socket protocol for communication between the app and `alp-infer`.

use alp_core::CoreError;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

use crate::TtsRequest;

/// Default Unix socket path used by the inference subsystem.
#[must_use]
pub fn default_socket_path() -> std::path::PathBuf {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
    std::path::PathBuf::from(runtime_dir).join("alp-infer.sock")
}

/// Request messages sent from the application / CLI to `alp-infer`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InferRequest {
    /// Keepalive / health check.
    Ping,
    /// Query current loaded model and engine status.
    GetStatus,
    /// Request the worker to load a model.
    LoadModel { model_id: String },
    /// Request speech synthesis for a single utterance.
    Synthesize { id: u64, request: TtsRequest },
    /// Request speech-to-text transcription for an audio buffer.
    Transcribe {
        id: u64,
        sample_rate: u32,
        channels: u16,
        samples: Vec<f32>,
    },
    /// Request the inference worker to design a voice profile from a text prompt.
    DesignVoice {
        id: u64,
        request: crate::design::VoiceDesignRequest,
    },
    /// Cooperatively cancel an in-flight synthesis request.
    Cancel { id: u64 },
    /// Request the inference worker to shut down gracefully.
    Shutdown,
}

/// Reply to `GetStatus` / `LoadModel`: which engines are active and whether
/// the last load succeeded.
///
/// `active_model` is the synthesis (TTS) tier; `active_stt_model` is the
/// transcription (STT) tier. The slots are independent: loading one does
/// not disturb the other.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InferStatus {
    /// Synthesis tier: the model currently serving `Synthesize`, or
    /// `builtin-acoustic` when the neural tier is not loaded.
    pub active_model: Option<String>,
    /// Whether the last `LoadModel` succeeded (the requested model is now
    /// active). `false` after a declined or failed load.
    pub ready: bool,
    /// Transcription tier: the STT model currently serving `Transcribe`,
    /// `None` while the built-in acoustic backend serves it.
    pub active_stt_model: Option<String>,
}

/// Response messages sent from `alp-infer` to the client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InferResponse {
    /// Reply to `Ping`.
    Pong,
    /// Reply to `GetStatus`.
    Status {
        active_model: Option<String>,
        ready: bool,
        active_stt_model: Option<String>,
    },
    /// Streaming progress update (0.0 to 1.0) for an ongoing request.
    Progress { id: u64, progress: f32 },
    /// Finished synthesized audio.
    Audio {
        id: u64,
        sample_rate: u32,
        channels: u16,
        samples: Vec<f32>,
    },
    /// Finished transcription segments.
    Transcript {
        id: u64,
        segments: Vec<crate::TranscriptSegment>,
    },
    /// Designed voice profile.
    VoiceDesigned {
        id: u64,
        profile: Box<crate::profile::VoiceProfile>,
    },
    /// Execution error.
    Error { id: u64, message: String },
}

/// Write a framed message: 4-byte LE length + JSON payload.
pub fn write_message<W: Write, T: Serialize>(writer: &mut W, msg: &T) -> Result<(), CoreError> {
    let json = serde_json::to_vec(msg)?;
    let len = json.len() as u32;
    writer.write_all(&len.to_le_bytes())?;
    writer.write_all(&json)?;
    writer.flush()?;
    Ok(())
}

/// Read a framed message: 4-byte LE length + JSON payload.
pub fn read_message<R: Read, T: for<'a> Deserialize<'a>>(reader: &mut R) -> Result<T, CoreError> {
    let mut len_bytes = [0u8; 4];
    reader.read_exact(&mut len_bytes)?;
    let len = u32::from_le_bytes(len_bytes) as usize;
    if len > 64 * 1024 * 1024 {
        return Err(CoreError::validation(
            "IPC message payload exceeds maximum limit",
        ));
    }

    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;

    serde_json::from_slice(&buf).map_err(CoreError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_round_trip() {
        let mut buf = Vec::new();
        let req = InferRequest::Synthesize {
            id: 42,
            request: TtsRequest {
                text: "Hello world".into(),
                voice_id: "voice1".into(),
                speed: 1.0,
            },
        };

        write_message(&mut buf, &req).expect("write");
        let decoded: InferRequest = read_message(&mut &buf[..]).expect("read");
        assert_eq!(req, decoded);
    }
}
