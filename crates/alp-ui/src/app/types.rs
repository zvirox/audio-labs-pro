//! Model and view data structures for the desktop application.

use std::path::PathBuf;

use alp_media::PcmBuffer;
use iced::Point;

use crate::panels::PanelKind;

#[derive(Debug, Clone, Copy)]
pub(super) enum Drag {
    Panel { kind: PanelKind, grab: Point },
    Resize { kind: PanelKind },
}

/// One effect item.
#[derive(Debug, Clone)]
pub(super) struct EffectItem {
    pub(super) name: String,
    pub(super) enabled: bool,
    pub(super) params: String,
}

/// A finished speech generation shown in the history feed.
#[derive(Debug, Clone)]
pub(super) struct HistoryItem {
    pub(super) text: String,
    pub(super) voice_name: String,
    pub(super) path: PathBuf,
    pub(super) duration_secs: f32,
    pub(super) created: String,
    pub(super) kind: &'static str,
}

/// A captured recording available in the Captures workspace.
#[derive(Debug, Clone)]
pub(super) struct CaptureItem {
    pub(super) name: String,
    pub(super) path: Option<PathBuf>,
    pub(super) duration_secs: f32,
    pub(super) transcript: String,
    pub(super) peak: f32,
}

/// Which background render the generation slot belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GenerationKind {
    /// A single spoken take.
    Take,
    /// A stitched story render.
    Story,
}

/// Output produced by a background TTS worker.
#[derive(Debug, Clone)]
pub(super) struct GeneratedTake {
    pub(super) text: String,
    pub(super) voice_name: String,
    pub(super) path: PathBuf,
    pub(super) duration_secs: f32,
    pub(super) kind: &'static str,
}

/// Output produced by a background render.
#[derive(Debug, Clone)]
pub(super) enum RenderOutput {
    /// One spoken take.
    Take(GeneratedTake),
    /// A stitched story plus each line's audio.
    Story {
        take: GeneratedTake,
        lines: Vec<(usize, PcmBuffer)>,
    },
}
