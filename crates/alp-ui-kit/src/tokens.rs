//! Design tokens shared by every Audio Lab Pro screen.
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
    pub const SM: u16 = 6;
    /// Default panel radius.
    pub const MD: u16 = 12;
    /// Floating panel radius.
    pub const LG: u16 = 18;
    /// Pill radius for toggles and tags.
    pub const PILL: u16 = 999;
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    /// Dark glass (default).
    Dark,
    /// Light glass.
    Light,
    /// Follow the desktop preference.
    System,
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
}
