//! Dark and light glass palettes.
//!
//! Every screen reads colors from [`ThemeColors`]; no widget owns a literal
//! color. The tint is deliberately opaque enough to stay legible when the
//! compositor cannot blur the desktop behind the window (ADR 0012).

use iced::widget::container;
use iced::{Background, Border, Color, Shadow, Theme, Vector, theme};

use crate::tokens::{Radius, ThemeMode};

/// Resolved colors for one theme.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemeColors {
    /// Window background when the desktop is not visible.
    pub background: Color,
    /// Panel glass tint.
    pub glass: Color,
    /// Stronger glass tint for raised cards and overlays.
    pub glass_strong: Color,
    /// Primary text.
    pub text: Color,
    /// Secondary text.
    pub muted: Color,
    /// Dim text and disabled content.
    pub faint: Color,
    /// Accent for primary actions and focus.
    pub accent: Color,
    /// Accent hover.
    pub accent_hover: Color,
    /// Accent pressed.
    pub accent_pressed: Color,
    /// Hairline border.
    pub border: Color,
    /// Strong border for a focused panel.
    pub border_strong: Color,
    /// Drop shadow.
    pub shadow: Color,
    /// Success and positive meters.
    pub success: Color,
    /// Warning badges.
    pub warning: Color,
    /// Destructive actions and errors.
    pub danger: Color,
    /// Waveform body.
    pub wave: Color,
    /// Waveform highlight.
    pub wave_hot: Color,
    /// Timeline grid.
    pub grid: Color,
    /// Playhead.
    pub playhead: Color,
}

impl ThemeColors {
    /// Dark glass palette.
    pub const DARK: Self = Self {
        background: Color::from_rgba(0.035, 0.040, 0.055, 0.92),
        glass: Color::from_rgba(0.055, 0.062, 0.080, 0.72),
        glass_strong: Color::from_rgba(0.075, 0.082, 0.105, 0.94),
        text: Color::from_rgba(0.95, 0.96, 1.0, 0.95),
        muted: Color::from_rgba(0.72, 0.75, 0.84, 0.78),
        faint: Color::from_rgba(0.62, 0.66, 0.76, 0.45),
        accent: Color::from_rgb(0.40, 0.54, 0.95),
        accent_hover: Color::from_rgb(0.46, 0.60, 1.0),
        accent_pressed: Color::from_rgb(0.34, 0.47, 0.86),
        border: Color::from_rgba(1.0, 1.0, 1.0, 0.12),
        border_strong: Color::from_rgba(0.52, 0.64, 1.0, 0.55),
        shadow: Color::from_rgba(0.0, 0.0, 0.0, 0.48),
        success: Color::from_rgb(0.42, 0.86, 0.66),
        warning: Color::from_rgb(0.96, 0.76, 0.38),
        danger: Color::from_rgb(0.94, 0.42, 0.46),
        wave: Color::from_rgb(0.38, 0.72, 0.95),
        wave_hot: Color::from_rgb(0.96, 0.54, 0.42),
        grid: Color::from_rgba(1.0, 1.0, 1.0, 0.07),
        playhead: Color::from_rgb(1.0, 0.42, 0.42),
    };

    /// Light glass palette.
    pub const LIGHT: Self = Self {
        background: Color::from_rgba(0.90, 0.92, 0.96, 0.94),
        glass: Color::from_rgba(1.0, 1.0, 1.0, 0.72),
        glass_strong: Color::from_rgba(1.0, 1.0, 1.0, 0.94),
        text: Color::from_rgba(0.08, 0.10, 0.14, 0.96),
        muted: Color::from_rgba(0.25, 0.29, 0.38, 0.75),
        faint: Color::from_rgba(0.30, 0.34, 0.42, 0.45),
        accent: Color::from_rgb(0.20, 0.36, 0.82),
        accent_hover: Color::from_rgb(0.26, 0.43, 0.92),
        accent_pressed: Color::from_rgb(0.16, 0.30, 0.72),
        border: Color::from_rgba(0.08, 0.10, 0.16, 0.14),
        border_strong: Color::from_rgba(0.20, 0.36, 0.82, 0.55),
        shadow: Color::from_rgba(0.05, 0.08, 0.16, 0.20),
        success: Color::from_rgb(0.10, 0.52, 0.36),
        warning: Color::from_rgb(0.66, 0.42, 0.06),
        danger: Color::from_rgb(0.72, 0.18, 0.22),
        wave: Color::from_rgb(0.18, 0.44, 0.80),
        wave_hot: Color::from_rgb(0.78, 0.30, 0.20),
        grid: Color::from_rgba(0.08, 0.10, 0.16, 0.08),
        playhead: Color::from_rgb(0.82, 0.16, 0.20),
    };

    /// Resolve a theme mode, including the desktop preference.
    #[must_use]
    pub fn for_mode(mode: ThemeMode) -> Self {
        match mode.resolve(crate::tokens::system_is_dark()) {
            ThemeMode::Light => Self::LIGHT,
            _ => Self::DARK,
        }
    }

    /// iced theme handed to built-in widgets.
    #[must_use]
    pub fn to_theme(self) -> Theme {
        Theme::custom(
            "Audio Labs Pro",
            theme::Palette {
                background: self.background,
                text: self.text,
                primary: self.accent,
                success: self.success,
                warning: self.warning,
                danger: self.danger,
            },
        )
    }

    /// Floating glass panel style.
    #[must_use]
    pub fn panel_style(self, focused: bool) -> container::Style {
        container::Style {
            background: Some(Background::Color(self.glass)),
            text_color: Some(self.text),
            border: Border {
                radius: f32::from(Radius::LG).into(),
                width: 1.0,
                color: if focused {
                    self.border_strong
                } else {
                    self.border
                },
            },
            shadow: Shadow {
                color: self.shadow,
                offset: Vector::new(0.0, 18.0),
                blur_radius: 48.0,
            },
            snap: true,
        }
    }

    /// Raised card inside a panel.
    #[must_use]
    pub fn card_style(self) -> container::Style {
        container::Style {
            background: Some(Background::Color(self.glass_strong)),
            text_color: Some(self.text),
            border: Border {
                radius: f32::from(Radius::MD).into(),
                width: 1.0,
                color: self.border,
            },
            shadow: Shadow::default(),
            snap: true,
        }
    }

    /// Transparent container style.
    #[must_use]
    pub fn plain_style(self) -> container::Style {
        container::Style {
            text_color: Some(self.text),
            ..container::Style::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glass_tint_is_legible_without_blur() {
        const { assert!(ThemeColors::DARK.glass.a >= 0.7) };
        const { assert!(ThemeColors::LIGHT.glass.a >= 0.7) };
        const { assert!(ThemeColors::DARK.text.a > 0.9) };
    }

    #[test]
    fn light_and_dark_are_distinct() {
        assert_ne!(ThemeColors::DARK.background, ThemeColors::LIGHT.background);
        assert_ne!(ThemeColors::DARK.text, ThemeColors::LIGHT.text);
    }
}
