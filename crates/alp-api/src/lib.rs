//! Automation service layer for Audio Labs Pro.
//!
//! Provides a typed, permission-tiered API for external AI clients, MCP agents,
//! and headless scripts to interact with projects, media, tracks, clips, DSP,
//! voice synthesis, transcription, and mastering export.

pub mod acx_qa;
pub mod analysis_ops;
pub mod audiobook;
pub mod audit;
pub mod checkpoint;
#[cfg(target_os = "linux")]
pub mod control;
pub mod dubbing_ops;
pub mod effect_ops;
pub mod error;
pub mod export_ops;
pub mod job_ops;
pub mod jobs;
pub mod macro_ops;
pub mod media;
pub mod paging;
pub mod project_ops;
pub mod repair_ops;
pub mod sandbox;
pub mod script_voice_ops;
pub mod session;
pub mod smart_edit_ops;
pub mod studio_ops;
pub mod track_clip_ops;
pub mod types;
pub mod voice;

pub use acx_qa::{
    ACX_NOISE_FLOOR_MAX_DBFS, ACX_RMS_MAX_DBFS, ACX_RMS_MIN_DBFS, ACX_TRUE_PEAK_MAX_DBFS,
    AcxChapterInput, AcxChapterQa, AcxQaReport, analyze_acx_chapters,
};
pub use alp_engine::{MacroRequest, MacroStep};
pub use alp_media::ExportPreset;
pub use audiobook::AudiobookEncodeReport;
pub use audit::{AuditEntry, AuditLogger};
pub use checkpoint::{Checkpoint, CheckpointManager};
pub use dubbing_ops::{DubbingFitReport, DubbingFitRequestDto, DubbingRemuxRequestDto};
pub use error::ApiError;
pub use jobs::{JobInfo, JobManager, JobState};
pub use macro_ops::MacroJobReport;
pub use media::{ClipPlacementReport, MediaImportReport, MediaListReport, MediaSummary};
pub use paging::{
    DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE, Page, PageRequest, decode_cursor, encode_cursor,
};
pub use project_ops::{HistoryEntry, ProjectListItem};
pub use repair_ops::{
    SpectralRepairReport, SpectralRepairRequestDto, TimeStretchReport, TimeStretchRequestDto,
};
pub use sandbox::WorkspaceSandbox;
pub use script_voice_ops::{ScriptGenerateReport, ScriptLine, ScriptRegenerateReport};
pub use session::{ApiSession, AttachedProjectState};
pub use studio_ops::{
    AcxChapterFileDto, EqBandDto, EqResponseDto, M4bChapterDto, M4bExportRequestDto,
    M4bMetadataDto, compute_eq_response,
};
pub use track_clip_ops::{
    BatchOp, ClipSlipReport, EditBatchReport, PanEnvelopeReport, SpectralSelectionReport,
    ViewModeReport,
};
pub use types::{
    AnalysisReport, ApprovalMode, ClipSummary, PermissionTier, ProjectSummary, TrackSummary,
    format_timestamp,
};
pub use voice::{
    BatchTtsReport, BatchTtsRequest, TranscriptionReport, TranscriptionSegmentReport,
    TtsGenerationReport, TtsVoicesReport, VoiceParamsDetail, VoiceParamsUpdate, VoiceProfileDetail,
    VoiceProfileUpdate, VoiceSummary,
};
