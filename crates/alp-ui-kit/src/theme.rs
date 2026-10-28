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
    /// Secondary accent for tags, links and alternate meters.
    pub accent_secondary: Color,
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
    /// Retro dark palette: walnut cabinet, cream lettering, burnt-orange accent.
    pub const DARK: Self = Self {
        background: Color::from_rgba(0.075, 0.062, 0.051, 0.98),
        glass: Color::from_rgba(0.106, 0.086, 0.067, 0.92),
        glass_strong: Color::from_rgba(0.137, 0.110, 0.086, 0.98),
        text: Color::from_rgba(0.949, 0.902, 0.812, 0.97),
        muted: Color::from_rgba(0.706, 0.639, 0.522, 0.90),
        faint: Color::from_rgba(0.478, 0.424, 0.341, 0.75),
        accent: Color::from_rgb(0.847, 0.514, 0.235),
        accent_hover: Color::from_rgb(0.925, 0.616, 0.314),
        accent_pressed: Color::from_rgb(0.702, 0.396, 0.157),
        accent_secondary: Color::from_rgb(0.373, 0.612, 0.573),
        border: Color::from_rgba(0.949, 0.902, 0.812, 0.11),
        border_strong: Color::from_rgba(0.847, 0.514, 0.235, 0.55),
        shadow: Color::from_rgba(0.0, 0.0, 0.0, 0.55),
        success: Color::from_rgb(0.545, 0.671, 0.400),
        warning: Color::from_rgb(0.890, 0.706, 0.267),
        danger: Color::from_rgb(0.769, 0.322, 0.243),
        wave: Color::from_rgb(0.847, 0.514, 0.235),
        wave_hot: Color::from_rgb(0.949, 0.741, 0.404),
        grid: Color::from_rgba(0.949, 0.902, 0.812, 0.07),
        playhead: Color::from_rgb(0.949, 0.741, 0.404),
    };

    /// Retro light palette: aged paper, ink lettering, oxblood accent.
    pub const LIGHT: Self = Self {
        background: Color::from_rgba(0.910, 0.867, 0.776, 0.97),
        glass: Color::from_rgba(0.965, 0.937, 0.875, 0.94),
        glass_strong: Color::from_rgba(0.984, 0.961, 0.910, 0.99),
        text: Color::from_rgba(0.180, 0.145, 0.106, 0.97),
        muted: Color::from_rgba(0.376, 0.325, 0.255, 0.85),
        faint: Color::from_rgba(0.541, 0.482, 0.388, 0.70),
        accent: Color::from_rgb(0.663, 0.278, 0.129),
        accent_hover: Color::from_rgb(0.749, 0.341, 0.161),
        accent_pressed: Color::from_rgb(0.549, 0.216, 0.094),
        accent_secondary: Color::from_rgb(0.184, 0.435, 0.412),
        border: Color::from_rgba(0.180, 0.145, 0.106, 0.18),
        border_strong: Color::from_rgba(0.663, 0.278, 0.129, 0.55),
        shadow: Color::from_rgba(0.200, 0.150, 0.100, 0.20),
        success: Color::from_rgb(0.247, 0.463, 0.259),
        warning: Color::from_rgb(0.706, 0.494, 0.086),
        danger: Color::from_rgb(0.659, 0.212, 0.161),
        wave: Color::from_rgb(0.663, 0.278, 0.129),
        wave_hot: Color::from_rgb(0.788, 0.427, 0.180),
        grid: Color::from_rgba(0.180, 0.145, 0.106, 0.10),
        playhead: Color::from_rgb(0.788, 0.427, 0.180),
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
                offset: Vector::new(0.0, 6.0),
                blur_radius: 18.0,
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
