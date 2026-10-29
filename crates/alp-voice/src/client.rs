//! Client connection to `alp-infer` and in-process acoustic synthesizer fallback.

use alp_core::CoreError;
use alp_media::PcmBuffer;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use crate::{
    TtsBackend, TtsRequest, VoiceError, VoiceInfo, VoiceKind,
    protocol::{InferRequest, InferResponse, default_socket_path, read_message, write_message},
    text::TextNormalizer,
};

/// High-level client communicating with the `alp-infer` worker over Unix domain sockets.
pub struct InferClient {
    socket_path: PathBuf,
    next_id: AtomicU64,
}

impl InferClient {
    /// Create a client using the default runtime socket path.
    #[must_use]
    pub fn new() -> Self {
        Self::with_socket(default_socket_path())
    }

    /// Create a client with an explicit socket path.
    #[must_use]
    pub fn with_socket(path: impl Into<PathBuf>) -> Self {
        Self {
            socket_path: path.into(),
            next_id: AtomicU64::new(1),
        }
    }

    /// Check if the inference daemon is actively responding.
    #[must_use]
    pub fn is_alive(&self) -> bool {
        self.ping().is_ok()
    }

    /// Ping the inference daemon.
    pub fn ping(&self) -> Result<(), CoreError> {
        let mut stream = UnixStream::connect(&self.socket_path)?;
        stream.set_read_timeout(Some(Duration::from_millis(500)))?;

        write_message(&mut stream, &InferRequest::Ping)?;
        let resp: InferResponse = read_message(&mut stream)?;
        match resp {
            InferResponse::Pong => Ok(()),
            other => Err(CoreError::validation(format!(
                "unexpected response: {other:?}"
            ))),
        }
    }

    /// Query the daemon's active model and engine readiness.
    pub fn status(&self) -> Result<(Option<String>, bool), CoreError> {
        let mut stream = UnixStream::connect(&self.socket_path)?;
        stream.set_read_timeout(Some(Duration::from_millis(500)))?;

        write_message(&mut stream, &InferRequest::GetStatus)?;
        let resp: InferResponse = read_message(&mut stream)?;
        match resp {
            InferResponse::Status {
                active_model,
                ready,
            } => Ok((active_model, ready)),
            other => Err(CoreError::validation(format!(
                "unexpected response: {other:?}"
            ))),
        }
    }

    /// Ask the daemon to load a model into its active engine.
    ///
    /// The returned `ready` refers to the *requested* model: `false` means the
    /// daemon declined (unknown model, load failure) and kept its previous
    /// engine.
    pub fn load_model(&self, model_id: &str) -> Result<(Option<String>, bool), CoreError> {
        let mut stream = UnixStream::connect(&self.socket_path)?;
        stream.set_read_timeout(Some(Duration::from_millis(500)))?;

        write_message(
            &mut stream,
            &InferRequest::LoadModel {
                model_id: model_id.to_string(),
            },
        )?;
        let resp: InferResponse = read_message(&mut stream)?;
        match resp {
            InferResponse::Status {
                active_model,
                ready,
            } => Ok((active_model, ready)),
            other => Err(CoreError::validation(format!(
                "unexpected response: {other:?}"
            ))),
        }
    }

    /// Send a synthesis request to the daemon.
    pub fn synthesize(&self, request: &TtsRequest) -> Result<PcmBuffer, VoiceError> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .map_err(|e| VoiceError::Unavailable(format!("alp-infer daemon unreachable: {e}")))?;

        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        write_message(
            &mut stream,
            &InferRequest::Synthesize {
                id,
                request: request.clone(),
            },
        )
        .map_err(|e| VoiceError::Backend(e.to_string()))?;

        loop {
            let resp: InferResponse = read_message(&mut stream)
                .map_err(|e| VoiceError::Backend(format!("IPC read failed: {e}")))?;

            match resp {
                InferResponse::Audio {
                    id: resp_id,
                    sample_rate,
                    channels,
                    samples,
                } if resp_id == id => {
                    return PcmBuffer::new(sample_rate, channels, samples)
                        .map_err(|e| VoiceError::Backend(e.to_string()));
                }
                InferResponse::Error {
                    id: resp_id,
                    message,
                } if resp_id == id => {
                    return Err(VoiceError::Backend(message));
                }
                InferResponse::Progress { .. } => {
                    // Ongoing progress update
                }
                _ => {}
            }
        }
    }

    /// Send a transcription request to the daemon.
    pub fn transcribe(&self, pcm: &PcmBuffer) -> Result<Vec<crate::TranscriptSegment>, VoiceError> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .map_err(|e| VoiceError::Unavailable(format!("alp-infer daemon unreachable: {e}")))?;

        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        write_message(
            &mut stream,
            &InferRequest::Transcribe {
                id,
                sample_rate: pcm.sample_rate(),
                channels: pcm.channels(),
                samples: pcm.samples().to_vec(),
            },
        )
        .map_err(|e| VoiceError::Backend(e.to_string()))?;

        loop {
            let resp: InferResponse = read_message(&mut stream)
                .map_err(|e| VoiceError::Backend(format!("IPC read failed: {e}")))?;

            match resp {
                InferResponse::Transcript {
                    id: resp_id,
                    segments,
                } if resp_id == id => {
                    return Ok(segments);
                }
                InferResponse::Error {
                    id: resp_id,
                    message,
                } if resp_id == id => {
                    return Err(VoiceError::Backend(message));
                }
                InferResponse::Progress { .. } => {}
                _ => {}
            }
        }
    }

    /// Send a voice design request to the daemon.
    pub fn design_voice(
        &self,
        request: &crate::design::VoiceDesignRequest,
    ) -> Result<crate::profile::VoiceProfile, VoiceError> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .map_err(|e| VoiceError::Unavailable(format!("alp-infer daemon unreachable: {e}")))?;

        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        write_message(
            &mut stream,
            &InferRequest::DesignVoice {
                id,
                request: request.clone(),
            },
        )
        .map_err(|e| VoiceError::Backend(e.to_string()))?;

        loop {
            let resp: InferResponse = read_message(&mut stream)
                .map_err(|e| VoiceError::Backend(format!("IPC read failed: {e}")))?;

            match resp {
                InferResponse::VoiceDesigned {
                    id: resp_id,
                    profile,
                } if resp_id == id => {
                    return Ok(*profile);
                }
                InferResponse::Error {
                    id: resp_id,
                    message,
                } if resp_id == id => {
                    return Err(VoiceError::Backend(message));
                }
                _ => {}
            }
        }
    }
}

impl Default for InferClient {
    fn default() -> Self {
        Self::new()
    }
}

/// In-process lightweight acoustic formant / harmonic speech synthesizer.
///
/// Ships embedded in the application binary so speech generation works offline
/// with zero downloads, zero dependencies, and instant startup.
#[derive(Debug, Clone)]
pub struct BuiltinAcousticBackend {
    sample_rate: u32,
}

impl BuiltinAcousticBackend {
    /// Create a built-in acoustic synthesizer at 48 kHz.
    #[must_use]
    pub fn new() -> Self {
        Self {
            sample_rate: 48_000,
        }
    }

    /// Create with a specific sample rate.
    #[must_use]
    pub fn with_sample_rate(sample_rate: u32) -> Self {
        Self { sample_rate }
    }
}

impl Default for BuiltinAcousticBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl TtsBackend for BuiltinAcousticBackend {
    fn id(&self) -> &'static str {
        "builtin-acoustic"
    }

    fn voices(&self) -> Vec<VoiceInfo> {
        vec![
            VoiceInfo {
                id: "aura".into(),
                name: "Aura (Clean)".into(),
                language: "en".into(),
                kind: VoiceKind::Builtin,
            },
            VoiceInfo {
                id: "atlas".into(),
                name: "Atlas (Warm)".into(),
                language: "en".into(),
                kind: VoiceKind::Builtin,
            },
            VoiceInfo {
                id: "echo".into(),
                name: "Echo (Clear)".into(),
                language: "en".into(),
                kind: VoiceKind::Builtin,
            },
            VoiceInfo {
                id: "sol".into(),
                name: "Sol (Bright)".into(),
                language: "en".into(),
                kind: VoiceKind::Builtin,
            },
        ]
    }

    fn synthesize(&self, request: &TtsRequest) -> Result<PcmBuffer, VoiceError> {
        let normalized = TextNormalizer::normalize(&request.text);
        if normalized.trim().is_empty() {
            return PcmBuffer::new(self.sample_rate, 1, vec![0.0; 100])
                .map_err(|e| VoiceError::Backend(e.to_string()));
        }

        // Voice characteristics: fundamental frequency (f0) and formant shifts
        let (f0_base, f1_shift, f2_shift) = match request.voice_id.as_str() {
            "atlas" => (110.0f32, 0.9f32, 0.92f32), // deeper male voice
            "aura" => (200.0f32, 1.15f32, 1.12f32), // warm female voice
            "echo" => (150.0f32, 1.0f32, 1.0f32),   // neutral clear voice
            "sol" => (240.0f32, 1.25f32, 1.2f32),   // bright voice
            _ => (160.0f32, 1.0f32, 1.0f32),
        };

        let speed = request.speed.clamp(0.5, 2.5);
        let phoneme_duration_ms = 90.0 / speed;
        let phoneme_frames = ((phoneme_duration_ms * 0.001) * self.sample_rate as f32) as usize;

        let words: Vec<&str> = normalized.split_whitespace().collect();
        let mut samples = Vec::new();

        // 50ms lead-in silence
        let lead_in = (0.05 * self.sample_rate as f32) as usize;
        samples.resize(lead_in, 0.0);

        let mut phase: f32 = 0.0;

        for (word_idx, word) in words.iter().enumerate() {
            for char_val in word.chars() {
                if !char_val.is_alphabetic() {
                    continue;
                }

                // Vowel formant mapping (F1, F2, F3)
                let (f1, f2, f3, is_voiced) = match char_val.to_ascii_lowercase() {
                    'a' => (800.0 * f1_shift, 1200.0 * f2_shift, 2500.0, true),
                    'e' => (500.0 * f1_shift, 1800.0 * f2_shift, 2600.0, true),
                    'i' => (300.0 * f1_shift, 2200.0 * f2_shift, 2900.0, true),
                    'o' => (500.0 * f1_shift, 900.0 * f2_shift, 2400.0, true),
                    'u' => (350.0 * f1_shift, 800.0 * f2_shift, 2300.0, true),
                    's' | 'f' | 't' | 'k' | 'p' => (0.0, 0.0, 4500.0, false), // unvoiced fricative/plosive
                    _ => (450.0 * f1_shift, 1400.0 * f2_shift, 2500.0, true), // voiced consonant
                };

                let dt = 1.0 / (self.sample_rate as f32);

                for frame_idx in 0..phoneme_frames {
                    let progress = frame_idx as f32 / phoneme_frames as f32;
                    // Envelope attack/decay
                    let env = if progress < 0.15 {
                        progress / 0.15
                    } else if progress > 0.85 {
                        (1.0 - progress) / 0.15
                    } else {
                        1.0
                    };

                    let sample = if is_voiced {
                        // Pulse train with harmonics through formant resonances
                        phase += f0_base * dt;
                        if phase >= 1.0 {
                            phase -= 1.0;
                        }

                        let fundamental = (phase * 2.0 * std::f32::consts::PI).sin();
                        let f1_res =
                            ((phase * (f1 / f0_base)) * 2.0 * std::f32::consts::PI).sin() * 0.45;
                        let f2_res =
                            ((phase * (f2 / f0_base)) * 2.0 * std::f32::consts::PI).sin() * 0.35;
                        let f3_res =
                            ((phase * (f3 / f0_base)) * 2.0 * std::f32::consts::PI).sin() * 0.20;

                        (fundamental * 0.4 + f1_res + f2_res + f3_res) * env * 0.35
                    } else {
                        // Shaped white noise for unvoiced consonants
                        let seed = ((word_idx * 1000 + frame_idx) as u32)
                            .wrapping_mul(1664525)
                            .wrapping_add(1013904223);
                        let noise = (seed as f32 / u32::MAX as f32) * 2.0 - 1.0;
                        noise * env * 0.18
                    };

                    samples.push(sample);
                }
            }

            // Word boundary gap (60ms)
            let word_gap = ((0.06 / speed) * self.sample_rate as f32) as usize;
            samples.resize(samples.len() + word_gap, 0.0);
        }

        // Tail padding (50ms)
        let tail_frames = (0.05 * self.sample_rate as f32) as usize;
        samples.resize(samples.len() + tail_frames, 0.0);

        PcmBuffer::new(self.sample_rate, 1, samples).map_err(|e| VoiceError::Backend(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spawn a fake daemon on a tempdir socket that answers each incoming
    /// request with the supplied response, for `accepts` connections.
    fn fake_daemon(
        reply: InferResponse,
        accepts: usize,
    ) -> (std::path::PathBuf, std::thread::JoinHandle<()>) {
        use crate::protocol::{read_message, write_message};
        use tempfile::tempdir;

        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("infer.sock");
        let listener = std::os::unix::net::UnixListener::bind(&path).expect("bind unix socket");
        let socket = path.clone();
        let handle = std::thread::spawn(move || {
            // The tempdir lives as long as this thread, keeping the socket file valid.
            let _dir_owned = dir;
            for _ in 0..accepts {
                let Ok((mut stream, _)) = listener.accept() else {
                    break;
                };
                stream
                    .set_read_timeout(Some(Duration::from_millis(2_000)))
                    .ok();
                let _request: InferRequest = read_message(&mut stream).expect("read request");
                let _ = write_message(&mut stream, &reply);
            }
        });
        (socket, handle)
    }

    #[test]
    fn status_and_load_model_round_trip_through_socket() {
        let (socket, handle) = fake_daemon(
            InferResponse::Status {
                active_model: Some("kokoro-82m".into()),
                ready: true,
            },
            2,
        );
        let client = InferClient::with_socket(&socket);

        let (active, ready) = client.status().expect("status");
        assert_eq!(active.as_deref(), Some("kokoro-82m"));
        assert!(ready);

        let (loaded, ready) = client.load_model("kokoro-82m").expect("load");
        assert!(loaded.is_some());
        assert!(ready);
        handle.join().expect("daemon thread");
    }

    #[test]
    fn ping_mismatched_reply_is_an_error() {
        let (socket, handle) = fake_daemon(
            InferResponse::Status {
                active_model: None,
                ready: false,
            },
            2,
        );
        let client = InferClient::with_socket(&socket);
        assert!(client.ping().is_err(), "a Status reply is not a Pong");
        // A Pong-shaped expectation against a Status reply stays an error on
        // the next call too; the client surfaces it instead of panicking.
        assert!(client.ping().is_err());
        handle.join().expect("daemon thread");
    }

    #[test]
    fn builtin_acoustic_synthesizes_valid_audio() {
        let backend = BuiltinAcousticBackend::new();
        let req = TtsRequest {
            text: "Hello from Audio Labs Pro".into(),
            voice_id: "aura".into(),
            speed: 1.0,
        };

        let pcm = backend.synthesize(&req).expect("synthesis");
        assert!(pcm.frames() > 10_000);
        assert_eq!(pcm.channels(), 1);
        assert_eq!(pcm.sample_rate(), 48_000);

        // Verify non-silent
        let peak = pcm
            .samples()
            .iter()
            .copied()
            .fold(0.0f32, |m, s| m.max(s.abs()));
        assert!(peak > 0.05, "expected audible sound, got peak {peak}");
    }
}
