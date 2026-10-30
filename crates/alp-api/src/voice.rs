//! Voice tool methods for `ApiSession`: voice listing and inspection, speech
//! synthesis into project media, voice design, and sandboxed transcription.
//!
//! These methods back the Phase 10 MCP voice toolset (ADR 0026). Engines run
//! daemon-first: an `InferClient` serves whichever tier is active in the
//! `alp-infer` worker, and the in-process built-in backends cover an absent
//! or failing daemon. Every result names the engine that actually ran, and
//! the built-in STT keeps its placeholder-words admission wherever it runs.

use std::path::PathBuf;

use alp_core::{frames_to_secs, now_ms};
use alp_media::{
    AudioMetadata, DitherMode, ExportFormat, ExportSettings, PcmBuffer, WavBitDepth, decode, export,
};
use alp_voice::{
    BuiltinAcousticBackend, BuiltinSttBackend, InferClient, ProfileStore, SttBackend, Transcript,
    TranscriptSegment, TtsBackend, TtsRequest, VoiceDesignRequest, VoiceKind, VoiceProfile,
    design_voice,
};
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::session::ApiSession;
use crate::types::PermissionTier;

/// Minimum accepted speech speed multiplier.
const TTS_SPEED_MIN: f32 = 0.25;
/// Maximum accepted speech speed multiplier.
const TTS_SPEED_MAX: f32 = 4.0;

/// One voice selectable for synthesis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceSummary {
    /// Stable profile identifier.
    pub id: String,
    /// Friendly display name.
    pub name: String,
    /// Voice origin: `builtin`, `designed` or `cloned`.
    pub kind: String,
    /// Backend engine that created and executes this voice.
    pub backend: String,
    /// Descriptive tags.
    pub tags: Vec<String>,
}

/// Result of the `tts_voices` tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TtsVoicesReport {
    /// Known voices in deterministic order (builtin, then designed, then cloned; id ascending).
    pub voices: Vec<VoiceSummary>,
    /// Whether the `alp-infer` daemon is reachable.
    pub daemon_alive: bool,
    /// Synthesis model currently loaded in the daemon, if any.
    pub daemon_active_model: Option<String>,
}

/// Generation parameters reported for a voice profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceParamsDetail {
    /// Speed multiplier (`1.0` is neutral).
    pub speed: f32,
    /// Pitch adjustment in semitones (`0.0` is neutral).
    pub pitch: f32,
    /// Energy / vocal effort (`1.0` is neutral).
    pub energy: f32,
    /// Expressive emotion / style tag.
    pub emotion: String,
}

/// Full public view of one voice profile.
///
/// Internal state (`cached_embedding`) and consent record contents are
/// deliberately not exposed; consent is reported as a boolean only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceProfileDetail {
    /// Stable profile identifier.
    pub id: String,
    /// Friendly display name.
    pub name: String,
    /// Voice origin: `builtin`, `designed` or `cloned`.
    pub kind: String,
    /// Backend engine that created and executes this voice.
    pub backend: String,
    /// Descriptive tags.
    pub tags: Vec<String>,
    /// Default generation parameters.
    pub default_params: VoiceParamsDetail,
    /// Unix creation timestamp (seconds).
    pub created_at: u64,
    /// Whether reference audio is stored with the profile.
    pub has_reference_audio: bool,
    /// Whether a consent record exists (contents stay private).
    pub consent_recorded: bool,
}

/// Result of the `tts_generate` tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TtsGenerationReport {
    /// Absolute path of the written WAV file (inside the project media dir).
    pub path: String,
    /// Audio length in seconds.
    pub duration_secs: f64,
    /// Engine that synthesized the audio: `daemon:<model>` or `builtin-acoustic`.
    pub engine: String,
    /// Sample rate of the written file in Hz.
    pub sample_rate: u32,
}

/// One transcribed region with positions in seconds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptionSegmentReport {
    /// Segment start in seconds.
    pub start_secs: f64,
    /// Segment end in seconds.
    pub end_secs: f64,
    /// Recognized text.
    pub text: String,
}

/// Result of the `transcribe` tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptionReport {
    /// Engine that transcribed the audio: `daemon:<model>`, `daemon:builtin`
    /// (daemon-side segmenter) or `builtin` (in-process segmenter, whose
    /// output is placeholder words).
    pub engine: String,
    /// Duration of the transcribed audio in seconds.
    pub duration_secs: f64,
    /// Plain-text transcription.
    pub text: String,
    /// Segments in chronological order.
    pub segments: Vec<TranscriptionSegmentReport>,
    /// Absolute path of the written transcript file when `out` was requested.
    pub transcript_path: Option<String>,
}

fn kind_label(kind: VoiceKind) -> &'static str {
    match kind {
        VoiceKind::Builtin => "builtin",
        VoiceKind::Designed => "designed",
        VoiceKind::Cloned => "cloned",
    }
}

fn kind_rank(label: &str) -> usize {
    match label {
        "builtin" => 0,
        "designed" => 1,
        _ => 2,
    }
}

fn voice_summary(profile: &VoiceProfile) -> VoiceSummary {
    VoiceSummary {
        id: profile.id.clone(),
        name: profile.name.clone(),
        kind: kind_label(profile.kind).to_owned(),
        backend: profile.backend.clone(),
        tags: profile.tags.clone(),
    }
}

fn voice_detail(profile: VoiceProfile) -> VoiceProfileDetail {
    VoiceProfileDetail {
        id: profile.id,
        name: profile.name,
        kind: kind_label(profile.kind).to_owned(),
        backend: profile.backend,
        tags: profile.tags,
        default_params: VoiceParamsDetail {
            speed: profile.default_params.speed,
            pitch: profile.default_params.pitch,
            energy: profile.default_params.energy,
            emotion: profile.default_params.emotion,
        },
        created_at: profile.created_at,
        has_reference_audio: profile.reference_audio.is_some(),
        consent_recorded: profile.consent.is_some(),
    }
}

/// Synthesize daemon-first; falls back to the in-process acoustic engine and
/// names the engine that actually produced audio.
fn synthesize_daemon_first(
    request: &TtsRequest,
    sample_rate: u32,
    engine_out: &mut String,
) -> Result<PcmBuffer, ApiError> {
    let client = InferClient::new();
    if client.is_alive() {
        match client.synthesize(request) {
            Ok(buffer) => {
                *engine_out = match client.status().ok().and_then(|s| s.active_model) {
                    Some(model) => format!("daemon:{model}"),
                    None => "daemon:builtin-acoustic".to_owned(),
                };
                return Ok(buffer);
            }
            Err(error) => {
                tracing::warn!(
                    "daemon synthesis failed ({error}); falling back to the built-in engine"
                );
            }
        }
    }
    *engine_out = "builtin-acoustic".to_owned();
    BuiltinAcousticBackend::with_sample_rate(sample_rate)
        .synthesize(request)
        .map_err(|e| ApiError::Validation(format!("synthesis failed: {e}")))
}

/// Transcribe daemon-first; falls back to the in-process segmenter and names
/// the engine that actually read the audio.
fn transcribe_daemon_first(
    buffer: &PcmBuffer,
    engine_out: &mut String,
) -> Result<Vec<TranscriptSegment>, ApiError> {
    let client = InferClient::new();
    if client.is_alive() {
        match client.transcribe(buffer) {
            Ok(segments) => {
                *engine_out = match client.status().ok().and_then(|s| s.active_stt_model) {
                    Some(model) => format!("daemon:{model}"),
                    None => "daemon:builtin".to_owned(),
                };
                return Ok(segments);
            }
            Err(error) => {
                tracing::warn!(
                    "daemon transcription failed ({error}); falling back to the built-in segmenter"
                );
            }
        }
    }
    *engine_out = "builtin".to_owned();
    BuiltinSttBackend::new()
        .transcribe(buffer)
        .map_err(|e| ApiError::Validation(format!("transcription failed: {e}")))
}

/// Filesystem-safe TTS output file name.
fn tts_file_name(voice_id: &str) -> String {
    let safe: String = voice_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(32)
        .collect();
    let safe = if safe.is_empty() {
        "voice".to_owned()
    } else {
        safe
    };
    let stamp = now_ms();
    format!("tts_{safe}_{stamp}.wav")
}

impl ApiSession {
    /// Override the voice profile store directory (tests and embedding hosts).
    /// Production sessions keep the default XDG user store.
    #[must_use]
    pub fn with_voice_store_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.voice_store_dir = Some(dir.into());
        self
    }

    /// Open the voice profile store: the session override directory when set,
    /// the default XDG user store otherwise.
    fn voice_store(&self) -> Result<ProfileStore, ApiError> {
        match &self.voice_store_dir {
            Some(dir) => ProfileStore::new(dir),
            None => ProfileStore::default_store(),
        }
        .map_err(ApiError::from)
    }

    /// List known voices plus daemon synthesis state (Phase 10 `tts_voices`).
    pub fn tts_voices(&self) -> Result<TtsVoicesReport, ApiError> {
        self.check_permission(PermissionTier::Observe)?;
        let store = self.voice_store()?;
        let mut voices: Vec<VoiceSummary> =
            store.list_profiles().iter().map(voice_summary).collect();
        voices.sort_by(|a, b| {
            kind_rank(&a.kind)
                .cmp(&kind_rank(&b.kind))
                .then_with(|| a.id.cmp(&b.id))
        });

        let client = InferClient::new();
        let (daemon_alive, daemon_active_model) = if client.is_alive() {
            (true, client.status().ok().and_then(|s| s.active_model))
        } else {
            (false, None)
        };

        Ok(TtsVoicesReport {
            voices,
            daemon_alive,
            daemon_active_model,
        })
    }

    /// Inspect one voice profile by id (Phase 10 `voice_profile_get`).
    pub fn voice_profile_get(&self, voice_id: &str) -> Result<VoiceProfileDetail, ApiError> {
        self.check_permission(PermissionTier::Observe)?;
        let store = self.voice_store()?;
        let profile = store.get_profile(voice_id).ok_or_else(|| {
            ApiError::NotFound(format!(
                "voice '{voice_id}' not found; call tts_voices for the known ids"
            ))
        })?;
        Ok(voice_detail(profile))
    }

    /// Synthesize speech into the project media directory (Phase 10
    /// `tts_generate`). Daemon-first with built-in fallback; writes a 16-bit
    /// WAV at the synthesis sample rate and reports the engine that ran.
    pub fn tts_generate(
        &mut self,
        text: &str,
        voice_id: &str,
        speed: Option<f32>,
    ) -> Result<TtsGenerationReport, ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        let text = text.trim();
        if text.is_empty() {
            return Err(ApiError::Validation(
                "text is empty; nothing to synthesize".into(),
            ));
        }

        let store = self.voice_store()?;
        if store.get_profile(voice_id).is_none() {
            return Err(ApiError::NotFound(format!(
                "voice '{voice_id}' not found; call tts_voices for the known ids"
            )));
        }

        let speed = speed.unwrap_or(1.0).clamp(TTS_SPEED_MIN, TTS_SPEED_MAX);
        let request = TtsRequest {
            text: text.to_owned(),
            voice_id: voice_id.to_owned(),
            speed,
        };

        let mut engine = String::from("builtin-acoustic");
        let buffer = synthesize_daemon_first(&request, self.project.sample_rate, &mut engine)?;

        let target = self
            .sandbox
            .validate_path(self.paths.media_dir().join(tts_file_name(voice_id)))?;
        let settings = ExportSettings {
            format: ExportFormat::Wav,
            bit_depth: WavBitDepth::Int16,
            sample_rate: Some(buffer.sample_rate()),
            dither: DitherMode::Triangular,
            metadata: AudioMetadata::default(),
        };
        export(&target, &buffer, &settings)?;

        let report = TtsGenerationReport {
            path: target.display().to_string(),
            duration_secs: buffer.duration_secs(),
            engine,
            sample_rate: buffer.sample_rate(),
        };
        self.record_audit("tts_generate", "ok", self.revision());
        Ok(report)
    }

    /// Design a new voice from a text brief and persist it (Phase 10
    /// `voice_design`). This path can only ever produce a `Designed` profile:
    /// cloned voice creation and import are human-only and unreachable here.
    pub fn voice_design(
        &mut self,
        name: &str,
        prompt: &str,
        tags: Vec<String>,
    ) -> Result<VoiceProfileDetail, ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        let name = name.trim();
        let prompt = prompt.trim();
        if name.is_empty() {
            return Err(ApiError::Validation("voice name is empty".into()));
        }
        if prompt.is_empty() {
            return Err(ApiError::Validation(
                "design prompt is empty; describe the voice, e.g. 'warm, calm narrator'".into(),
            ));
        }

        let profile = design_voice(&VoiceDesignRequest {
            name: name.to_owned(),
            prompt: prompt.to_owned(),
            tags,
        });
        debug_assert!(
            profile.kind == VoiceKind::Designed && profile.consent.is_none(),
            "the design path must never create cloned profiles or consent records"
        );

        let store = self.voice_store()?;
        store.save_profile(&profile)?;
        let saved = voice_detail(profile);
        self.record_audit("voice_design", "ok", self.revision());
        Ok(saved)
    }

    /// Transcribe a sandboxed audio file (Phase 10 `transcribe`). Daemon-first
    /// with built-in fallback; optionally writes `text`/`srt`/`vtt` output.
    pub fn transcribe(
        &mut self,
        path: &str,
        format: Option<&str>,
        out: Option<&str>,
    ) -> Result<TranscriptionReport, ApiError> {
        self.check_permission(PermissionTier::Observe)?;

        let written_format = format
            .map(|f| f.to_ascii_lowercase())
            .unwrap_or_else(|| "text".to_owned());
        if !matches!(written_format.as_str(), "text" | "srt" | "vtt") {
            return Err(ApiError::Validation(format!(
                "unknown transcript format '{written_format}': use text, srt or vtt"
            )));
        }

        let input = self.sandbox.validate_path(path)?;
        let buffer = decode(&input)?;

        let mut engine = String::from("builtin");
        let segments = transcribe_daemon_first(&buffer, &mut engine)?;
        let transcript = Transcript { segments };
        let text = transcript.to_plain_text();

        let mut transcript_path = None;
        if let Some(out) = out {
            let target = self.sandbox.validate_path(out)?;
            let content = match written_format.as_str() {
                "srt" => transcript.to_srt(buffer.sample_rate()),
                "vtt" => transcript.to_vtt(buffer.sample_rate()),
                _ => transcript.to_plain_text(),
            };
            std::fs::write(&target, content)?;
            transcript_path = Some(target.display().to_string());
        }

        let rate = buffer.sample_rate();
        let report = TranscriptionReport {
            duration_secs: buffer.duration_secs(),
            engine,
            segments: transcript
                .segments
                .iter()
                .map(|s| TranscriptionSegmentReport {
                    start_secs: frames_to_secs(s.start_frame, rate),
                    end_secs: frames_to_secs(s.end_frame, rate),
                    text: s.text.clone(),
                })
                .collect(),
            text,
            transcript_path,
        };
        self.record_audit("transcribe", "ok", self.revision());
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::AuditLogger;
    use crate::sandbox::WorkspaceSandbox;
    use alp_core::{Project, ProjectPaths};
    use tempfile::tempdir;

    fn session_in(dir: &std::path::Path, tier: PermissionTier) -> ApiSession {
        let proj_paths = ProjectPaths::new(dir.join("proj.alp"));
        proj_paths.ensure_layout().expect("layout");
        let project = Project::new("Voice Test").expect("project");
        let sandbox = WorkspaceSandbox::new([dir]).expect("sandbox");
        let audit = AuditLogger::new(dir.join("audit.jsonl")).expect("audit");
        ApiSession::new(project, proj_paths, tier, sandbox, audit, "agent")
            .expect("session")
            .with_voice_store_dir(dir.join("voices"))
    }

    #[test]
    fn voices_list_is_deterministic_and_seeded() {
        let dir = tempdir().expect("tempdir");
        let session = session_in(dir.path(), PermissionTier::Edit);

        let first = session.tts_voices().expect("voices");
        let second = session.tts_voices().expect("voices");
        // Only the list itself is deterministic; daemon liveness depends on
        // the machine and is compared separately.
        assert_eq!(
            first.voices, second.voices,
            "the voice list must be deterministic"
        );

        let ids: Vec<&str> = first.voices.iter().map(|v| v.id.as_str()).collect();
        assert!(ids.contains(&"aura"), "built-in seed voices must be listed");
        let ranks: Vec<usize> = first.voices.iter().map(|v| kind_rank(&v.kind)).collect();
        assert!(
            ranks.windows(2).all(|w| w[0] <= w[1]),
            "builtin voices sort first"
        );
    }

    #[test]
    fn unknown_voice_profile_is_not_found() {
        let dir = tempdir().expect("tempdir");
        let session = session_in(dir.path(), PermissionTier::Edit);
        let error = session
            .voice_profile_get("does-not-exist")
            .expect_err("unknown id must fail");
        assert!(matches!(error, ApiError::NotFound(_)));
    }

    #[test]
    fn tts_generate_writes_wav_into_media_and_names_engine() {
        let dir = tempdir().expect("tempdir");
        let mut session = session_in(dir.path(), PermissionTier::Edit);

        let report = session
            .tts_generate("Hello from the agent", "aura", Some(3.0))
            .expect("generate");
        assert!(
            report.path.ends_with(".wav"),
            "v1 writes WAV: {}",
            report.path
        );
        assert!(
            report.path.contains("media"),
            "output lands in the media dir"
        );
        assert!(std::path::Path::new(&report.path).is_file(), "file written");
        assert!(report.duration_secs > 0.0, "duration reported");

        // Environment-agnostic honesty: a reachable daemon names itself,
        // otherwise the in-process built-in engine does.
        assert!(
            report.engine == "builtin-acoustic" || report.engine.starts_with("daemon:"),
            "engine must be named honestly: {}",
            report.engine
        );
    }

    #[test]
    fn tts_generate_rejects_unknown_voice_and_empty_text() {
        let dir = tempdir().expect("tempdir");
        let mut session = session_in(dir.path(), PermissionTier::Edit);

        let unknown = session
            .tts_generate("hello", "no-such-voice", None)
            .expect_err("unknown");
        assert!(matches!(unknown, ApiError::NotFound(_)));

        let empty = session
            .tts_generate("   ", "aura", None)
            .expect_err("empty");
        assert!(matches!(empty, ApiError::Validation(_)));
    }

    #[test]
    fn observe_tier_cannot_generate_or_design() {
        let dir = tempdir().expect("tempdir");
        let mut session = session_in(dir.path(), PermissionTier::Observe);
        let error = session
            .tts_generate("hello", "aura", None)
            .expect_err("tier");
        assert!(matches!(error, ApiError::PermissionDenied(_)));
    }

    #[test]
    fn transcribe_round_trip_is_honest_about_the_engine() {
        let dir = tempdir().expect("tempdir");
        let mut session = session_in(dir.path(), PermissionTier::Edit);

        let tts = session
            .tts_generate("Testing transcription round trip", "aura", None)
            .expect("generate");
        let out = dir.path().join("out.srt");
        let report = session
            .transcribe(&tts.path, Some("srt"), out.to_str())
            .expect("transcribe");

        // Environment-agnostic honesty, mirroring the UI admission.
        if report.engine == "builtin" {
            assert!(
                report.text.contains("Speech segment"),
                "built-in STT must keep the placeholder-words admission"
            );
        } else {
            assert!(
                report.engine.starts_with("daemon:"),
                "engine: {}",
                report.engine
            );
        }
        assert!(report.duration_secs > 0.0);

        let written = std::fs::read_to_string(&out).expect("srt file");
        assert!(written.contains("-->"), "srt timestamps present");
        assert_eq!(
            report.transcript_path.as_deref(),
            Some(out.to_str().expect("utf8"))
        );
    }

    #[test]
    fn transcribe_rejects_unknown_format_and_escape_paths() {
        let dir = tempdir().expect("tempdir");
        let mut session = session_in(dir.path(), PermissionTier::Edit);
        let tts = session
            .tts_generate("Format guard test", "aura", None)
            .expect("generate");

        let bad_format = session
            .transcribe(&tts.path, Some("xml"), None)
            .expect_err("format");
        assert!(matches!(bad_format, ApiError::Validation(_)));

        let escape = session
            .transcribe("../escape.wav", None, None)
            .expect_err("traversal");
        assert!(
            matches!(escape, ApiError::SandboxEscape(_)),
            "input paths must stay inside the sandbox"
        );
    }

    #[test]
    fn voice_design_lifecycle_stays_designed_only() {
        let dir = tempdir().expect("tempdir");
        let mut session = session_in(dir.path(), PermissionTier::Edit);

        let designed = session
            .voice_design(
                "Calm Guide",
                "A calm, deep documentary narrator",
                vec!["narration".into()],
            )
            .expect("design");
        assert_eq!(
            designed.kind, "designed",
            "the MCP path only designs voices"
        );
        assert!(!designed.consent_recorded, "consent stays human-only");

        let voices = session.tts_voices().expect("voices");
        assert!(
            voices.voices.iter().all(|v| v.kind != "cloned"),
            "no tool in this slice may produce a cloned profile"
        );
        assert!(
            voices
                .voices
                .iter()
                .any(|v| v.id == designed.id && v.kind == "designed"),
            "the designed voice is listed"
        );

        let got = session.voice_profile_get(&designed.id).expect("get");
        assert_eq!(got.id, designed.id);
        assert!(
            got.default_params.pitch < 0.0,
            "the 'deep' brief lands in the params"
        );

        let empty_prompt = session
            .voice_design("No Brief", "   ", vec![])
            .expect_err("empty prompt");
        assert!(matches!(empty_prompt, ApiError::Validation(_)));
    }
}
