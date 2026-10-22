//! `alp-voice` — TTS, STT and cloning traits plus the model manifest types.
//!
//! Phase 1 defines the boundary only. Backends arrive with the model manager in
//! Phase 6 and the cloning tier in Phase 8. Implementations live in `alp-infer`
//! and reach the app over the Unix-socket protocol.

use alp_media::PcmBuffer;

/// Error produced by a voice backend.
#[derive(Debug, thiserror::Error)]
pub enum VoiceError {
    /// The requested voice is not provided by the backend.
    #[error("unknown voice {0}")]
    UnknownVoice(String),
    /// The backend is not available (missing model, failed to start).
    #[error("backend unavailable: {0}")]
    Unavailable(String),
    /// Synthesis or transcription failed.
    #[error("backend failure: {0}")]
    Backend(String),
}

/// Description of one selectable voice.
#[derive(Debug, Clone, PartialEq)]
pub struct VoiceInfo {
    /// Backend-local identifier.
    pub id: String,
    /// Friendly name.
    pub name: String,
    /// BCP-47 language tag; v1 ships `en`.
    pub language: String,
    /// Whether the voice is built in, cloned or designed.
    pub kind: VoiceKind,
}

/// Origin of a voice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceKind {
    /// Shipped with a lightweight model.
    Builtin,
    /// Created from a reference recording.
    Cloned,
    /// Generated from a text description.
    Designed,
}

/// One synthesis request.
#[derive(Debug, Clone, PartialEq)]
pub struct TtsRequest {
    /// Text to speak.
    pub text: String,
    /// Voice identifier.
    pub voice_id: String,
    /// Speed multiplier (`1.0` is neutral).
    pub speed: f32,
}

/// A text-to-speech backend.
pub trait TtsBackend: Send {
    /// Stable backend identifier.
    fn id(&self) -> &str;

    /// Voices this backend can speak with.
    fn voices(&self) -> Vec<VoiceInfo>;

    /// Synthesize a request into a PCM buffer.
    fn synthesize(&self, request: &TtsRequest) -> Result<PcmBuffer, VoiceError>;
}

/// A backend that can create reusable cloned voices.
pub trait CloneBackend: TtsBackend {
    /// Create a profile from a reference recording and optional transcript.
    fn create_profile(
        &self,
        reference: &PcmBuffer,
        transcript: Option<&str>,
    ) -> Result<VoiceProfile, VoiceError>;

    /// Synthesize using an existing profile.
    fn synthesize_with(
        &self,
        profile: &VoiceProfile,
        request: &TtsRequest,
    ) -> Result<PcmBuffer, VoiceError>;
}

/// A reusable voice profile, independent of any project.
#[derive(Debug, Clone, PartialEq)]
pub struct VoiceProfile {
    /// Stable profile id.
    pub id: String,
    /// Friendly name.
    pub name: String,
    /// Built-in, cloned or designed.
    pub kind: VoiceKind,
    /// Backend that produced and can use it.
    pub backend: String,
    /// Attestation that the speaker consented to cloning.
    pub consent_record: Option<String>,
}

/// A transcription segment with timestamps in frames.
#[derive(Debug, Clone, PartialEq)]
pub struct TranscriptSegment {
    /// Start frame in the input buffer.
    pub start_frame: u64,
    /// End frame in the input buffer.
    pub end_frame: u64,
    /// Recognized text.
    pub text: String,
}

/// A speech-to-text backend.
pub trait SttBackend: Send {
    /// Transcribe a complete buffer.
    fn transcribe(&self, pcm: &PcmBuffer) -> Result<Vec<TranscriptSegment>, VoiceError>;

    /// Whether the backend supports partial results.
    fn streaming(&self) -> bool;
}
