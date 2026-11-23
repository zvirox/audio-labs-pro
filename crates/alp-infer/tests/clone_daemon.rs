//! End-to-end verification of the clone tier over the real daemon socket.
//!
//! This test starts the actual `alp-infer` binary, talks to it over a Unix
//! domain socket with the production protocol, and checks the clone path
//! end to end: a profile with a measured voiceprint is sent to the daemon,
//! the daemon renders it, and the returned audio is re-analyzed to confirm it
//! carries the reference's pitch.
//!
//! It also pins the consent gate: the daemon must refuse a cloned profile
//! that arrives without a consent record.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use alp_media::PcmBuffer;
use alp_voice::{
    AcousticCloneBackend, CloneBackend, ConsentRecord, InferClient, TtsRequest, VoiceKind,
    VoiceProfile, analyze_voiceprint,
};

/// A voiced vowel with a known fundamental.
fn vowel(f0: f32, seconds: f32, rate: u32) -> PcmBuffer {
    let frames = (seconds * rate as f32) as usize;
    let samples: Vec<f32> = (0..frames)
        .map(|index| {
            let t = index as f32 / rate as f32;
            let fundamental = (2.0 * std::f32::consts::PI * f0 * t).sin();
            let second = 0.55 * (2.0 * std::f32::consts::PI * 2.0 * f0 * t).sin();
            let third = 0.30 * (2.0 * std::f32::consts::PI * 3.0 * f0 * t).sin();
            0.25 * (fundamental + second + third) / 1.85
        })
        .collect();
    PcmBuffer::new(rate, 1, samples).expect("pcm")
}

/// A phrase-like sequence of vowels with pauses.
fn phrase(f0: f32, rate: u32) -> PcmBuffer {
    let mut samples = Vec::new();
    for seconds in [0.6f32, 0.5, 0.6, 0.5] {
        let chunk = vowel(f0, seconds, rate);
        samples.extend_from_slice(chunk.samples());
        samples.extend(std::iter::repeat_n(0.0, (0.12 * rate as f32) as usize));
    }
    PcmBuffer::new(rate, 1, samples).expect("pcm")
}

/// The daemon process, killed when the guard drops.
struct Daemon {
    child: Child,
    socket: PathBuf,
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Start the daemon on a private socket and wait for it to accept clients.
fn start_daemon() -> Option<Daemon> {
    start_daemon_with_qwen(None)
}

/// Start the real worker, optionally overriding its Qwen model directory for
/// an explicit model-gated integration run.
fn start_daemon_with_qwen(qwen_dir: Option<&Path>) -> Option<Daemon> {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let dir = tempfile::tempdir().expect("tempdir");
    // The socket must outlive the `tempdir`, so it lives in its own
    // directory that is removed by the OS rather than by `TempDir`.
    let socket = std::env::temp_dir().join(format!(
        "alp-infer-test-{}-{}-{}.sock",
        std::process::id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_alp-infer"));

    let mut command = Command::new(&binary);
    command
        .arg("--socket")
        .arg(&socket)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(model_dir) = qwen_dir {
        command.env("ALP_QWEN3_DIR", model_dir);
    }
    let child = command.spawn().expect("spawn alp-infer");

    let daemon = Daemon { child, socket };

    // Wait for the socket to answer a ping.
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if daemon.socket.exists() {
            let client = InferClient::with_socket(&daemon.socket);
            if client.is_alive() {
                // Keep the socket path; drop the tempdir.
                drop(dir);
                return Some(daemon);
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    None
}

#[test]
fn the_daemon_renders_a_cloned_profile_with_its_voiceprint() {
    let Some(daemon) = start_daemon() else {
        eprintln!("skipping: the alp-infer daemon did not start");
        return;
    };

    // Build a profile with a measured voiceprint and a consent record.
    let mut profile = AcousticCloneBackend::new()
        .create_profile(&phrase(115.0, 48_000), Some("reference phrase"))
        .expect("create profile");
    profile.id = "daemon-clone-test".into();
    profile.consent = Some(
        ConsentRecord::new(
            "Test Speaker",
            "Test Engineer",
            "The speaker gave explicit permission to clone this voice.",
        )
        .expect("consent"),
    );
    assert_eq!(profile.kind, VoiceKind::Cloned);

    let client = InferClient::with_socket(&daemon.socket);
    let mut alive = false;
    for _ in 0..20 {
        if client.is_alive() {
            alive = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(alive, "the daemon must answer a ping");

    let request = TtsRequest {
        text: "The daemon renders this cloned voice".into(),
        voice_id: profile.id.clone(),
        speed: 1.0,
    };
    let rendered = client
        .synthesize_with_profile(&profile, &request)
        .expect("daemon clone synthesis");

    assert!(rendered.frames() > 10_000, "the render must not be empty");
    assert!(rendered.peak() > 0.05, "the render must be audible");

    // The audio must carry the reference's pitch.
    let print = analyze_voiceprint(&rendered);
    assert!(
        print.frames > 10,
        "the daemon's render must contain voiced frames, got {}",
        print.frames
    );
    assert!(
        (print.f0_hz - 115.0).abs() < 12.0,
        "the daemon's clone should speak near 115 Hz, got {:.2}",
        print.f0_hz
    );
}

#[test]
fn the_daemon_refuses_a_cloned_profile_without_consent() {
    let Some(daemon) = start_daemon() else {
        eprintln!("skipping: the alp-infer daemon did not start");
        return;
    };

    let mut profile = AcousticCloneBackend::new()
        .create_profile(&phrase(115.0, 48_000), None)
        .expect("create profile");
    profile.id = "daemon-clone-no-consent".into();
    profile.consent = None;

    let client = InferClient::with_socket(&daemon.socket);
    let request = TtsRequest {
        text: "This must not render".into(),
        voice_id: profile.id.clone(),
        speed: 1.0,
    };
    let error = client
        .synthesize_with_profile(&profile, &request)
        .expect_err("the daemon must refuse");
    let message = error.to_string();
    assert!(
        message.contains("consent"),
        "the refusal must name the consent gate: {message}"
    );
}

#[test]
fn the_daemon_executes_real_qwen_cloning_and_preserves_consent_when_installed() {
    let (Ok(model_dir), Ok(reference_path)) = (
        std::env::var("ALP_QWEN3_DIR"),
        std::env::var("ALP_QWEN3_REFERENCE"),
    ) else {
        eprintln!(
            "skip: set ALP_QWEN3_DIR and ALP_QWEN3_REFERENCE for real neural socket inference"
        );
        return;
    };
    let Some(daemon) = start_daemon_with_qwen(Some(Path::new(&model_dir))) else {
        panic!("the model-gated alp-infer daemon did not start");
    };
    let client = InferClient::with_socket(&daemon.socket);
    let loaded = client.load_model("qwen3-tts").expect("load real Qwen");
    assert!(loaded.ready, "the verified official model must load");
    assert_eq!(loaded.active_clone_model.as_deref(), Some("qwen3-tts"));
    assert_eq!(
        loaded.active_model.as_deref(),
        Some("builtin-acoustic"),
        "the clone slot must not rewrite plain-TTS provenance"
    );

    let mut profile = VoiceProfile::builtin("qwen-socket-test", "Qwen Socket Test", vec![]);
    profile.kind = VoiceKind::Cloned;
    profile.backend = "qwen3-tts".into();
    profile.reference_audio = Some(PathBuf::from(reference_path));
    profile.transcript = Some(
        "Okay. Yeah. I resent you. I love you. I respect you. But you know what? You blew it! And thanks to you."
            .into(),
    );
    let request = TtsRequest {
        text: "Hello world. This is a neural voice cloning test.".into(),
        voice_id: profile.id.clone(),
        speed: 1.0,
    };

    // Exercise the gate before consent is attached, while the neural model is
    // loaded. Consent must be refused by the daemon, not only by front ends.
    let refusal = client
        .synthesize_with_profile(&profile, &request)
        .expect_err("neural cloning without consent must be refused");
    assert!(refusal.to_string().contains("consent"), "{refusal}");

    profile.consent = Some(
        ConsentRecord::new(
            "Official Reference Speaker",
            "Test Engineer",
            "The speaker gave explicit permission to clone this voice.",
        )
        .expect("consent"),
    );
    let rendered = client
        .synthesize_with_profile(&profile, &request)
        .expect("real neural synthesis through the daemon");
    assert_eq!(rendered.sample_rate(), 24_000);
    assert_eq!(rendered.channels(), 1);
    assert!(rendered.frames() > 24_000, "expected more than one second");
    assert!(rendered.rms() > 1.0e-5, "output must not be silent");
    assert!(rendered.samples().iter().all(|sample| sample.is_finite()));

    let status = client.status().expect("status after neural synthesis");
    assert_eq!(status.active_clone_model.as_deref(), Some("qwen3-tts"));
    assert_eq!(status.active_model.as_deref(), Some("builtin-acoustic"));

    if let Ok(output) = std::env::var("ALP_QWEN3_OUTPUT") {
        alp_media::write_wav(output, &rendered, alp_media::WavBitDepth::Float32)
            .expect("write integration evidence WAV");
    }
}

#[test]
fn the_daemon_still_serves_plain_synthesis() {
    let Some(daemon) = start_daemon() else {
        eprintln!("skipping: the alp-infer daemon did not start");
        return;
    };

    // The clone slot must not disturb the synthesis slot.
    let client = InferClient::with_socket(&daemon.socket);
    let request = TtsRequest {
        text: "Plain synthesis still works".into(),
        voice_id: "aura".into(),
        speed: 1.0,
    };
    let rendered = client.synthesize(&request).expect("plain synthesis");
    assert!(rendered.frames() > 1000, "the render must not be empty");
}
