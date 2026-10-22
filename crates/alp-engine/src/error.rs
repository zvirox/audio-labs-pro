//! Error type for the audio engine.

use alp_core::CoreError;
use alp_media::MediaError;

/// Errors produced by `alp-engine`.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// No audio device matching the request is available.
    #[error("no audio device: {0}")]
    NoDevice(String),

    /// cpal failed to build or start a stream.
    #[error("audio stream error: {0}")]
    Stream(String),

    /// The device format cannot be used by the engine.
    #[error("unsupported device format: {0}")]
    UnsupportedFormat(String),

    /// The scene is inconsistent.
    #[error("invalid scene: {0}")]
    InvalidScene(String),

    /// Transport state rejected the operation.
    #[error("transport error: {0}")]
    Transport(String),

    /// Media failure while loading or rendering.
    #[error(transparent)]
    Media(#[from] MediaError),

    /// Core model failure.
    #[error(transparent)]
    Core(#[from] CoreError),

    /// Filesystem failure.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
