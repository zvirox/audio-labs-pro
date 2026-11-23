//! Floating panel model and iced adapters.

pub use alp_session::panels::*;

/// Extension trait for panel glyph icons.
pub trait PanelKindGlyph {
    /// Short icon glyph used in compact menus.
    fn glyph(self) -> alp_ui_kit::icons::Glyph;
}

impl PanelKindGlyph for PanelKind {
    fn glyph(self) -> alp_ui_kit::icons::Glyph {
        use alp_ui_kit::icons;
        match self {
            Self::Mixer => icons::MIXER,
            Self::Transport => icons::PLAY,
            Self::Timeline => icons::ROWS,
            Self::Inspector => icons::TARGET,
            Self::MediaBin => icons::GRID,
            Self::VoiceLab => icons::MIC,
            Self::Effects => icons::EFFECTS,
            Self::Models => icons::MODELS,
            Self::ExportQueue => icons::DOWNLOAD,
            Self::AiActivity => icons::AI,
        }
    }
}
