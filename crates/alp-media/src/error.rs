//! Error type for media decode, resample and cache operations.

use alp_core::CoreError;

/// Errors produced by `alp-media`.
#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    /// Filesystem failure.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The container or codec could not be opened.
    #[error("decode error: {0}")]
    Decode(String),

    /// The file uses a feature this build does not support.
    #[error("unsupported media: {0}")]
    Unsupported(String),

    /// A PCM buffer had inconsistent dimensions.
    #[error("invalid PCM buffer: {0}")]
    InvalidBuffer(String),

    /// A peak cache could not be read or was corrupt.
    #[error("invalid peak cache: {0}")]
    InvalidCache(String),

    /// Resampling failed.
    #[error("resample error: {0}")]
    Resample(String),

    /// WAV encoding failed.
    #[error("encode error: {0}")]
    Encode(String),

    /// Core model failure.
    #[error(transparent)]
    Core(#[from] CoreError),
}
