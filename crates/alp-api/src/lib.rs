//! Automation service layer for Audio Labs Pro.
//!
//! Provides a typed, permission-tiered API for external AI clients, MCP agents,
//! and headless scripts to interact with projects, media, tracks, clips, DSP,
//! voice synthesis, transcription, and mastering export.

pub mod audit;
pub mod checkpoint;
pub mod error;
pub mod jobs;
pub mod sandbox;
pub mod session;
pub mod types;

pub use alp_media::ExportPreset;
pub use audit::{AuditEntry, AuditLogger};
pub use checkpoint::{Checkpoint, CheckpointManager};
pub use error::ApiError;
pub use jobs::{JobInfo, JobManager, JobState};
pub use sandbox::WorkspaceSandbox;
pub use session::ApiSession;
pub use types::{
    AnalysisReport, ApprovalMode, ClipSummary, PermissionTier, ProjectSummary, TrackSummary,
    format_timestamp,
};
