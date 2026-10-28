//! Error types for CLAP and LV2 plugin hosting and execution.

use std::path::PathBuf;
use thiserror::Error;

/// Specific error conditions encountered during plugin discovery and execution.
#[derive(Debug, Error)]
pub enum PluginError {
    /// Plugin file or bundle was not found at the expected location.
    #[error("plugin not found: {0}")]
    NotFound(String),

    /// Plugin scanning or path traversal failed.
    #[error("plugin discovery error on {path}: {message}")]
    Discovery { path: PathBuf, message: String },

    /// Dynamic library loading failed.
    #[error("dynamic library error loading {path}: {message}")]
    LibraryLoad { path: PathBuf, message: String },

    /// Plugin ABI or specification version mismatch.
    #[error("incompatible plugin ABI for {plugin_id}: {message}")]
    AbiMismatch { plugin_id: String, message: String },

    /// Required entrypoint symbol was missing from the shared library.
    #[error("missing symbol '{symbol}' in plugin library {path}")]
    MissingSymbol { path: PathBuf, symbol: String },

    /// Failed to instantiate plugin.
    #[error("failed to instantiate plugin '{plugin_id}': {message}")]
    Instantiation { plugin_id: String, message: String },

    /// Audio activation failed.
    #[error("failed to activate plugin '{plugin_id}': {message}")]
    Activation { plugin_id: String, message: String },

    /// Real-time audio processing block failed.
    #[error("processing error in plugin '{plugin_id}': {message}")]
    Processing { plugin_id: String, message: String },

    /// Parameter identifier was not found in plugin descriptors.
    #[error("unknown parameter id {param_id} for plugin '{plugin_id}'")]
    UnknownParameter { plugin_id: String, param_id: u32 },

    /// Plugin state serialization or deserialization failed.
    #[error("state error for plugin '{plugin_id}': {message}")]
    State { plugin_id: String, message: String },

    /// Core domain error propagated into plugin hosting.
    #[error("core error: {0}")]
    Core(#[from] alp_core::CoreError),

    /// Standard I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
