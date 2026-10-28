//! Typed errors for the Audio Labs Pro automation service layer.

use alp_core::CoreError;
use thiserror::Error;

/// API errors encountered during automation calls.
#[derive(Debug, Error)]
pub enum ApiError {
    /// Requested entity not found.
    #[error("entity not found: {0}")]
    NotFound(String),

    /// Argument or precondition validation failed.
    #[error("validation error: {0}")]
    Validation(String),

    /// Optimistic concurrency conflict (expected revision did not match current project state).
    #[error(
        "concurrency revision conflict: expected revision {expected}, but project is at {actual}. Re-read project state and retry."
    )]
    Conflict { expected: u64, actual: u64 },

    /// Operation requires a higher permission tier than granted.
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    /// Action was refused because it violates workspace sandbox boundaries.
    #[error("sandbox security violation: {0}")]
    SandboxEscape(String),

    /// Human confirmation was required but not provided or denied.
    #[error("human confirmation required: {0}")]
    ConfirmationRequired(String),

    /// Underlying core error.
    #[error("core error: {0}")]
    Core(#[from] CoreError),

    /// Underlying media error.
    #[error("media error: {0}")]
    Media(#[from] alp_media::MediaError),

    /// Underlying I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error.
    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),
}
