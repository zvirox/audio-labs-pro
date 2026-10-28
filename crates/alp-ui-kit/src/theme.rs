//! Dark and light studio palettes.
//!
//! Every screen reads colors from [`ThemeColors`]; no widget owns a literal
//! color. The dark palette is the studio default: near-black surfaces, a warm
//! gold accent and hairline white borders. The light palette mirrors it with
//! paper surfaces and the same gold, so one component set serves both.

use iced::widget::container;
use iced::{Background, Border, Color, Shadow, Theme, Vector};

use crate::tokens::{Radius, ThemeMode};

/// Resolved colors for one theme.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemeColors {
    /// Window background behind the application frame.
    pub background: Color,
    /// Rail and sidebar tint.
    pub glass: Color,
    /// Card and raised-panel surface.
    pub glass_strong: Color,
    /// Inputs, wells and hovered rows.
    pub surface_raised: Color,
    /// Primary text.
    pub text: Color,
    /// Secondary text.
    pub muted: Color,
    /// Dim text and disabled content.
    pub faint: Color,
    /// Accent for primary actions and focus.
    pub accent: Color,
    /// Ink that stays legible on the accent fill.
    pub accent_text: Color,
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
    /// Modal scrim.
    pub scrim: Color,
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
    /// Studio dark: near-black frame, graphite cards, warm gold accent.
    pub const DARK: Self = Self {
        background: Color::from_rgb(0.039, 0.039, 0.039),
        glass: Color::from_rgb(0.055, 0.055, 0.055),
        glass_strong: Color::from_rgb(0.078, 0.078, 0.078),
        surface_raised: Color::from_rgb(0.106, 0.106, 0.106),
        text: Color::from_rgb(0.960, 0.960, 0.960),
        muted: Color::from_rgb(0.620, 0.620, 0.620),
        faint: Color::from_rgb(0.400, 0.400, 0.400),
        accent: Color::from_rgb(0.780, 0.651, 0.294),
        accent_text: Color::from_rgb(0.055, 0.047, 0.027),
        accent_hover: Color::from_rgb(0.847, 0.729, 0.400),
        accent_pressed: Color::from_rgb(0.651, 0.522, 0.204),
        accent_secondary: Color::from_rgb(0.655, 0.647, 0.620),
        border: Color::from_rgba(1.0, 1.0, 1.0, 0.10),
        border_strong: Color::from_rgba(0.780, 0.651, 0.294, 0.55),
        shadow: Color::from_rgba(0.0, 0.0, 0.0, 0.50),
        scrim: Color::from_rgba(0.0, 0.0, 0.0, 0.72),
        success: Color::from_rgb(0.478, 0.769, 0.541),
        warning: Color::from_rgb(0.878, 0.698, 0.361),
        danger: Color::from_rgb(0.878, 0.424, 0.373),
        wave: Color::from_rgb(0.780, 0.651, 0.294),
        wave_hot: Color::from_rgb(0.949, 0.831, 0.545),
        grid: Color::from_rgba(1.0, 1.0, 1.0, 0.055),
        playhead: Color::from_rgb(0.960, 0.960, 0.960),
    };

    /// Studio light: paper surfaces, graphite ink, the same gold accent.
    pub const LIGHT: Self = Self {
        background: Color::from_rgb(0.910, 0.910, 0.910),
        glass: Color::from_rgb(0.898, 0.898, 0.898),
        glass_strong: Color::from_rgb(0.969, 0.969, 0.969),
        surface_raised: Color::from_rgb(1.0, 1.0, 1.0),
        text: Color::from_rgb(0.078, 0.078, 0.078),
        muted: Color::from_rgb(0.430, 0.430, 0.430),
        faint: Color::from_rgb(0.600, 0.600, 0.600),
        accent: Color::from_rgb(0.678, 0.545, 0.227),
        accent_text: Color::from_rgb(1.0, 1.0, 1.0),
        accent_hover: Color::from_rgb(0.745, 0.612, 0.278),
        accent_pressed: Color::from_rgb(0.588, 0.463, 0.176),
        accent_secondary: Color::from_rgb(0.380, 0.380, 0.380),
        border: Color::from_rgba(0.0, 0.0, 0.0, 0.12),
        border_strong: Color::from_rgba(0.678, 0.545, 0.227, 0.65),
        shadow: Color::from_rgba(0.0, 0.0, 0.0, 0.16),
        scrim: Color::from_rgba(0.0, 0.0, 0.0, 0.45),
        success: Color::from_rgb(0.208, 0.518, 0.290),
        warning: Color::from_rgb(0.663, 0.467, 0.114),
        danger: Color::from_rgb(0.741, 0.243, 0.212),
        wave: Color::from_rgb(0.678, 0.545, 0.227),
        wave_hot: Color::from_rgb(0.808, 0.663, 0.320),
        grid: Color::from_rgba(0.0, 0.0, 0.0, 0.07),
        playhead: Color::from_rgb(0.078, 0.078, 0.078),
    };

    /// Resolve a theme mode, including the desktop preference.
    #[must_use]
    pub fn for_mode(mode: ThemeMode) -> Self {
        match mode.resolve(crate::tokens::system_is_dark()) {
            ThemeMode::Light => Self::LIGHT,
            _ => Self::DARK,
        }
    }

    /// iced theme handed to built-in widgets and to `iced-cube` controls.
    ///
    /// The palette is built through `iced_cube::theme::Config` so every
    /// iced-cube component resolves the same graphite and gold tokens from
    /// the active theme: the gold accent becomes the cube `primary`,
    /// raised surfaces become its secondary and hover accents, and the
    /// studio border hairline becomes its edge colour.
    #[must_use]
    pub fn to_theme(self) -> Theme {
        let luminance =
            0.2126 * self.background.r + 0.7152 * self.background.g + 0.0722 * self.background.b;
        let base = if luminance > 0.5 {
            iced_cube::theme::Config::light()
        } else {
            iced_cube::theme::Config::dark()
        };
        base.name("Audio Labs Pro")
            .background(self.background)
            .foreground(self.text)
            .primary(self.accent)
            .secondary(self.surface_raised)
            .accent(self.surface_raised)
            .destructive(self.danger)
            .success(self.success)
            .warning(self.warning)
            .border(self.border)
            .build()
    }

    /// The rounded application frame.
    #[must_use]
    pub fn frame_style(self) -> container::Style {
        container::Style {
            background: Some(Background::Color(self.glass)),
            text_color: Some(self.text),
            border: Border {
                radius: f32::from(Radius::XL).into(),
                width: 1.0,
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.07),
            },
            shadow: Shadow {
                color: self.shadow,
                offset: Vector::new(0.0, 18.0),
                blur_radius: 48.0,
            },
            snap: true,
        }
    }

    /// The fixed navigation rail.
    #[must_use]
    pub fn rail_style(self) -> container::Style {
        container::Style {
            background: Some(Background::Color(self.glass)),
            text_color: Some(self.text),
            border: Border {
                radius: 0.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            shadow: Shadow::default(),
            snap: true,
        }
    }

    /// Floating dock, dialog or overlay panel.
    #[must_use]
    pub fn panel_style(self, focused: bool) -> container::Style {
        container::Style {
            background: Some(Background::Color(self.glass_strong)),
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
                offset: Vector::new(0.0, 8.0),
                blur_radius: 24.0,
            },
            snap: true,
        }
    }

    /// Flat raised card inside the workspace.
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

    /// Recessed well for inputs, rulers and empty states.
    #[must_use]
    pub fn well_style(self) -> container::Style {
        container::Style {
            background: Some(Background::Color(self.surface_raised)),
            text_color: Some(self.text),
            border: Border {
                radius: f32::from(Radius::SM).into(),
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
    fn surfaces_stay_opaque_and_legible() {
        const { assert!(ThemeColors::DARK.glass_strong.a == 1.0) };
        const { assert!(ThemeColors::LIGHT.glass_strong.a == 1.0) };
        const { assert!(ThemeColors::DARK.text.a > 0.9) };
        const { assert!(ThemeColors::DARK.accent_text.r < 0.1) };
    }

    #[test]
    fn light_and_dark_are_distinct() {
        assert_ne!(ThemeColors::DARK.background, ThemeColors::LIGHT.background);
        assert_ne!(ThemeColors::DARK.text, ThemeColors::LIGHT.text);
    }
}
