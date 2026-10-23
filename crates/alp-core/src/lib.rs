//! `alp-core` — the project model, commands and persistence.
//!
//! This crate owns the data that defines a project and has no dependency on any
//! other ALP crate. See `docs/modules/alp-core.md`.
//!
//! ```
//! use alp_core::{CommandStack, Project, Track, TrackId};
//!
//! let mut project = Project::new("demo").unwrap();
//! let mut stack = CommandStack::new();
//! stack
//!     .execute(
//!         &mut project,
//!         Box::new(alp_core::AddTrack::new(
//!             Track::new(TrackId::new(), "Voice"),
//!             0,
//!         )),
//!     )
//!     .unwrap();
//! assert_eq!(project.tracks.len(), 1);
//! ```

mod clipboard;
mod command;
mod commands;
mod config;
mod edit_commands;
mod error;
mod files;
mod ids;
mod logging;
mod markers;
mod project;
mod project_dir;
mod schema;
mod selection;
mod time;

pub use clipboard::{ClipFragment, Clipboard, copy_range};
pub use command::{Command, CommandError, CommandStack};
pub use commands::{
    AddClip, AddMarker, AddRegion, AddSource, AddTrack, MoveClip, MoveTrack, RemoveClip,
    RemoveMarker, RemoveRegion, RemoveSource, RemoveTrack, RenameMarker, RenameProject,
    RenameRegion, RenameTrack, SetClipEffects, SetClipEnvelope, SetClipFades, SetClipGain,
    SetLoopRange, SetTrackArm, SetTrackColor, SetTrackGain, SetTrackMuted, SetTrackPan,
    SetTrackSolo, SplitClip, TrimClip,
};
pub use config::AppDirs;
pub use edit_commands::{
    DeleteRange, DuplicateClip, NormalizeClip, PasteClips, PunchIn, SetClipInverted,
    SetClipReversed, SilenceRange,
};
pub use error::CoreError;
pub use files::{commit_temp, create_temp_sibling, write_atomic};
pub use ids::{ClipId, MarkerId, ProjectId, RegionId, SourceId, TrackId};
pub use logging::{DEFAULT_FILTER, init as init_logging};
pub use markers::{Marker, Region};
pub use project::{
    Clip, DEFAULT_SAMPLE_RATE, EffectRef, EnvelopePoint, MAX_SAMPLE_RATE, MIN_SAMPLE_RATE, Project,
    Source, Track, now_ms,
};
pub use project_dir::ProjectPaths;
pub use schema::{CURRENT_SCHEMA_VERSION, from_json, schema_version, to_json};
pub use selection::Selection;
pub use time::{FrameRange, frames_to_secs, secs_to_frames};
