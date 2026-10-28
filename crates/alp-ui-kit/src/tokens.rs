//! Design tokens shared by every Audio Labs Pro screen.
//!
//! Values are unitless logical pixels or milliseconds. No screen may hard-code
//! its own spacing, radius or duration (build-plan rule 0.4.8).

/// Spacing scale in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spacing;

impl Spacing {
    /// 2 px.
    pub const XXS: u16 = 2;
    /// 4 px.
    pub const XS: u16 = 4;
    /// 8 px.
    pub const SM: u16 = 8;
    /// 12 px.
    pub const MD: u16 = 12;
    /// 16 px.
    pub const LG: u16 = 16;
    /// 24 px.
    pub const XL: u16 = 24;
    /// 32 px.
    pub const XXL: u16 = 32;
}

/// Corner radii in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Radius;

impl Radius {
    /// Small control radius.
    pub const SM: u16 = 3;
    /// Default panel radius.
    pub const MD: u16 = 7;
    /// Floating panel radius.
    pub const LG: u16 = 11;
    /// Pill radius for toggles and tags.
    pub const PILL: u16 = 999;
}

/// Control sizing in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Control;

impl Control {
    /// Compact row height.
    pub const SM: u16 = 28;
    /// Default control height.
    pub const MD: u16 = 34;
    /// Large primary action height.
    pub const LG: u16 = 42;
    /// Panel title bar height.
    pub const TITLE_BAR: u16 = 34;
    /// Resize grip hit area.
    pub const GRIP: u16 = 14;
    /// Custom window chrome height (title, panel menu and controls).
    pub const CHROME: u16 = 46;
}

/// Motion durations in milliseconds (ADR 0007).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Duration;

impl Duration {
    /// Hover and press feedback.
    pub const FAST_MS: u16 = 120;
    /// Default transition.
    pub const BASE_MS: u16 = 200;
    /// Panel open, close and settle.
    pub const PANEL_MS: u16 = 320;
}

/// Motion policy for one app session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Motion {
    /// When true, transitions are instantaneous.
    pub reduce_motion: bool,
}

impl Motion {
    /// Honour the Reduce motion setting.
    #[must_use]
    pub fn duration_ms(self, duration_ms: u16) -> u16 {
        if self.reduce_motion { 0 } else { duration_ms }
    }
}

/// Light or dark theme selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    /// Dark glass (default).
    #[default]
    Dark,
    /// Light glass.
    Light,
    /// Follow the desktop preference.
    System,
}

impl ThemeMode {
    /// Stable string used by the workspace file.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
            Self::System => "system",
        }
    }

    /// Parse a persisted mode, falling back to dark.
    #[must_use]
    pub fn parse(value: &str) -> Self {
        match value {
            "light" => Self::Light,
            "system" => Self::System,
            _ => Self::Dark,
        }
    }

    /// Resolve a system preference.
    #[must_use]
    pub fn resolve(self, system_dark: bool) -> Self {
        match self {
            Self::System if system_dark => Self::Dark,
            Self::System => Self::Light,
            other => other,
        }
    }

    /// Cycle dark → light → system → dark.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::System,
            Self::System => Self::Dark,
        }
    }
}

/// Type scale in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeScale;

impl TypeScale {
    /// Small labels.
    pub const CAPTION: u16 = 12;
    /// Body text.
    pub const BODY: u16 = 14;
    /// Emphasized body text.
    pub const BODY_STRONG: u16 = 15;
    /// Section titles.
    pub const TITLE: u16 = 20;
    /// Screen titles.
    pub const DISPLAY: u16 = 28;
}

/// Detect a dark desktop preference from the environment.
///
/// iced does not expose the portal preference in 0.14, so this uses the two
/// reliable Linux hints and falls back to dark. Callers can override with
/// `ALP_SYSTEM_THEME=light|dark`.
#[must_use]
pub fn system_is_dark() -> bool {
    if let Ok(value) = std::env::var("ALP_SYSTEM_THEME") {
        return value.eq_ignore_ascii_case("dark");
    }
    if let Ok(theme) = std::env::var("GTK_THEME") {
        let lower = theme.to_ascii_lowercase();
        if lower.contains("dark") {
            return true;
        }
        if lower.contains("light") {
            return false;
        }
    }
    if let Ok(value) = std::env::var("COLORFGBG") {
        // `fg;bg`; 0-6 are dark, 8-15 are light.
        if let Some(background) = value.rsplit(';').next()
            && let Ok(index) = background.parse::<u8>()
        {
            return index < 8;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reduce_motion_shortens_transitions() {
        assert_eq!(
            Motion {
                reduce_motion: false
            }
            .duration_ms(Duration::PANEL_MS),
            320
        );
        assert_eq!(
            Motion {
                reduce_motion: true
            }
            .duration_ms(Duration::PANEL_MS),
            0
        );
    }

    #[test]
    fn theme_mode_round_trips_and_cycles() {
        for mode in [ThemeMode::Dark, ThemeMode::Light, ThemeMode::System] {
            assert_eq!(ThemeMode::parse(mode.as_str()), mode);
        }
        assert_eq!(ThemeMode::Dark.next(), ThemeMode::Light);
        assert_eq!(ThemeMode::Light.next(), ThemeMode::System);
        assert_eq!(ThemeMode::System.next(), ThemeMode::Dark);
        assert_eq!(ThemeMode::System.resolve(true), ThemeMode::Dark);
        assert_eq!(ThemeMode::System.resolve(false), ThemeMode::Light);
    }

    #[test]
    fn unknown_theme_mode_falls_back_to_dark() {
        assert_eq!(ThemeMode::parse("solarized"), ThemeMode::Dark);
    }
}
