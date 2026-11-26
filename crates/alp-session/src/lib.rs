//! Project editing session, worker queue, and workspace persistence for Audio Labs Pro.
//!
//! `alp-session` is the headless coordination layer for desktop applications (both the
//! legacy iced interface and the target Tauri 2 / React shell). It owns:
//! - [`EditSession`]: active project, timeline commands, undo/redo stack, and audio sources.
//! - [`JobQueue`]: non-blocking background workers for exports, peak builds, and audio decoding.
//! - [`PersistedWorkspace`]: layout, theme, and window preferences persisted to disk.
//! - [`PanelManager`]: headless floating/docked panel geometry and z-order.
//! - [`TransportController`]: audio playback coordination, meter streaming, and peak pyramids.

pub mod ai;
pub mod audiobook_mastering;
pub mod dubbing;
pub mod effects;
pub mod jobs;
pub mod macro_bridge;
pub mod panels;
pub mod session;
pub mod timeline;
pub mod transport;
pub mod workspace;

pub use ai::{AiActivityEntry, AiControlManager, AiControlStatus, AiEvent};
pub use alp_api::AcxQaReport;
pub use alp_api::PermissionTier;
pub use alp_api::ProjectSummary;
pub use alp_api::control::ApprovalDecision;
pub use alp_api::{MacroJobReport, MacroRequest, MacroStep};
pub use alp_dsp::spectral::{FrequencyScale, SpectrogramConfig, SpectrogramData, WindowType};
pub use alp_media::{M4bExportReport, RemuxReport};
pub use audiobook_mastering::{
    AcxChapterFileDto, M4bChapterDto, M4bExportRequestDto, M4bMetadataDto, analyze_acx_files,
    export_audiobook_m4b,
};
pub use dubbing::{
    DUB_STEM_NAMES, RateFitReport, RateFitRequestDto, StemFileDto, StemSeparationReport,
    StemSeparationRequestDto, VideoRemuxRequestDto, fit_rate, remux_video, separate_stems,
};
pub use effects::{BiquadBandDto, BiquadResponseDto, compute_biquad_response};
pub use jobs::{Job, JobQueue, JobReporter, JobStatus};
pub use macro_bridge::dsp_macro_execute;
pub use panels::{DockSide, PanelKind, PanelManager, PanelRect, PanelState, Point, Size};
pub use session::{EditSession, ImportedAudio, NORMALIZE_TARGET};
pub use timeline::{
    SpectralSelectionDto, SpectrogramRequestDto, TimelineClipDto, TimelineDto,
    TimelineEnvelopePointDto, TimelineLoopRangeDto, TimelineMarkerDto, TimelinePanPointDto,
    TimelineSelectionDto, TimelineTrackDto, TrackViewMode,
};
pub use transport::{
    MeterFrame, PeakLevelDto, TransportController, TransportState, WaveformPeaksDto,
};
pub use workspace::{LAYOUT_VERSION, PersistedWorkspace};
