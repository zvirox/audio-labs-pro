//! `alp-voice` — TTS, STT and cloning traits, model manager, protocol and script mode.

pub mod cache;
pub mod client;
pub mod compare;
pub mod design;
pub mod hardware;
pub mod lexicon;
pub mod manager;
pub mod model;
pub mod profile;
pub mod protocol;
pub mod reference;
pub mod script;
pub mod sha256;
pub mod stt;
pub mod text;
pub mod transcript;
pub mod vad;
pub mod zip;

pub use cache::VoiceGenerationCache;
pub use client::{BuiltinAcousticBackend, InferClient};
pub use compare::{VoiceComparison, compare_audio};
pub use design::{VoiceDesignRequest, design_voice};
pub use hardware::{HardwareProfile, ModelTier};
pub use lexicon::{MarkupParser, PronunciationDictionary, SpeechToken};
pub use manager::{ModelManager, ModelManagerConfig};
pub use model::{
    ModelFile, ModelId, ModelKind, ModelLicense, ModelManifest, ModelStatus, standard_catalogue,
};
pub use profile::{ConsentRecord, ProfileStore, VoiceParams, VoiceProfile};
pub use protocol::{InferRequest, InferResponse, default_socket_path};
pub use reference::{ReferenceQualityReport, analyze_reference_quality};
pub use script::{LineStatus, Script, ScriptLine};
pub use stt::{BuiltinSttBackend, DictationEvent, DictationSession};
pub use text::TextNormalizer;
pub use transcript::Transcript;
pub use vad::{VadConfig, VoiceActivityDetector, segment_speech};

use alp_media::PcmBuffer;
use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoiceKind {
    /// Shipped with a lightweight model.
    Builtin,
    /// Created from a reference recording.
    Cloned,
    /// Generated from a text description.
    Designed,
}

/// One synthesis request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

/// A transcription segment with timestamps in frames.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
