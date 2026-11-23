//! Automation service layer for Audio Labs Pro.
//!
//! Provides a typed, permission-tiered API for external AI clients, MCP agents,
//! and headless scripts to interact with projects, media, tracks, clips, DSP,
//! voice synthesis, transcription, and mastering export.

pub mod analysis_ops;
pub mod audiobook;
pub mod audit;
pub mod checkpoint;
#[cfg(target_os = "linux")]
pub mod control;
pub mod effect_ops;
pub mod error;
pub mod export_ops;
pub mod job_ops;
pub mod jobs;
pub mod media;
pub mod paging;
pub mod project_ops;
pub mod sandbox;
pub mod script_voice_ops;
pub mod session;
pub mod smart_edit_ops;
pub mod track_clip_ops;
pub mod types;
pub mod voice;

pub use alp_media::ExportPreset;
pub use audiobook::AudiobookEncodeReport;
pub use audit::{AuditEntry, AuditLogger};
pub use checkpoint::{Checkpoint, CheckpointManager};
pub use error::ApiError;
pub use jobs::{JobInfo, JobManager, JobState};
pub use media::{ClipPlacementReport, MediaImportReport, MediaListReport, MediaSummary};
pub use paging::{
    DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE, Page, PageRequest, decode_cursor, encode_cursor,
};
pub use project_ops::{HistoryEntry, ProjectListItem};
pub use sandbox::WorkspaceSandbox;
pub use script_voice_ops::{ScriptGenerateReport, ScriptLine, ScriptRegenerateReport};
pub use session::{ApiSession, AttachedProjectState};
pub use track_clip_ops::{BatchOp, EditBatchReport};
pub use types::{
    AnalysisReport, ApprovalMode, ClipSummary, PermissionTier, ProjectSummary, TrackSummary,
    format_timestamp,
};
pub use voice::{
    BatchTtsReport, BatchTtsRequest, TranscriptionReport, TranscriptionSegmentReport,
    TtsGenerationReport, TtsVoicesReport, VoiceParamsDetail, VoiceParamsUpdate, VoiceProfileDetail,
    VoiceProfileUpdate, VoiceSummary,
};
