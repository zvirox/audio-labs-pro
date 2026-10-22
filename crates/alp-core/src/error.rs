//! Core error type shared by the project model, schema and persistence.

/// Errors produced by `alp-core`.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    /// A project sample rate outside the supported range.
    #[error("unsupported sample rate {0} Hz (expected 8000..=384000)")]
    InvalidSampleRate(u32),

    /// A frame range whose end precedes its start.
    #[error("invalid frame range: start {start} > end {end}")]
    InvalidRange {
        /// First frame of the range.
        start: u64,
        /// One past the last frame of the range.
        end: u64,
    },

    /// The project failed a structural validation check.
    #[error("invalid project: {0}")]
    Validation(String),

    /// The on-disk project schema is newer than this build understands.
    #[error("unsupported project schema version {found} (this build supports {supported})")]
    UnsupportedSchemaVersion {
        /// Version found in the file.
        found: u32,
        /// Newest version this build can load.
        supported: u32,
    },

    /// The schema version field is missing or not an integer.
    #[error("project file has no valid schema_version")]
    MissingSchemaVersion,

    /// The schema envelope was malformed.
    #[error("invalid project file: {0}")]
    InvalidProjectFile(String),

    /// Filesystem failure.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON encoding or decoding failure.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// XDG directories could not be resolved.
    #[error("could not resolve {0} directory")]
    MissingDirectory(&'static str),

    /// Logging setup failed.
    #[error("logging setup failed: {0}")]
    Logging(String),
}

impl CoreError {
    /// Convenience constructor for validation failures.
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }
}
