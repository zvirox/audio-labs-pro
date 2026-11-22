//! Dark and light studio palettes.
//!
//! Every screen reads colors from [`ThemeColors`]; no widget owns a literal
//! color. The dark palette is the studio default: calm slate surfaces,
//! high-contrast text and a restrained accent that the shell swaps per
//! workspace. The light palette uses paper surfaces and deepened accents so
//! one component set serves both modes.

use iced::widget::container;
use iced::{Background, Border, Color, Shadow, Theme, Vector};

use crate::tokens::{Radius, ThemeMode};

/// Restrained workspace accent hues, one per shell workspace.
///
/// Hues are named for the color itself and the shell maps each workspace onto
/// one. The vivid set is calibrated for dark slate surfaces and the deepened
/// set for paper light. Every derived color ([`ThemeColors::accent_hover`],
/// waveform hot, ink) comes from the base, so a hue cannot drift out of sync.
/// All accent/ink pairs are checked against the surface palette in the tests
/// below.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Neon {
    /// Electric cyan — synthesis and creation.
    Cyan,
    /// Neon magenta — narrative and stories.
    Magenta,
    /// Acid green — capture and recording.
    Acid,
    /// Electric violet — voice and identity.
    Violet,
    /// Blaster orange — effects and processing energy.
    Ember,
    /// Deep-sky blue — models and machine intelligence.
    Volt,
    /// Solar yellow — system controls and settings.
    Solar,
}

impl Neon {
    /// Every hue, in workspace order.
    pub const ALL: [Self; 7] = [
        Self::Cyan,
        Self::Magenta,
        Self::Acid,
        Self::Violet,
        Self::Ember,
        Self::Volt,
        Self::Solar,
    ];

    /// Base accent for a dark or light surface.
    ///
    /// Values are picked so `iced-cube`'s automatic ink rule (dark ink above a
    /// raw luminance of 0.55) keeps filled controls legible without the kit or
    /// the shell ever choosing ink on its own.
    #[must_use]
    pub const fn base(self, dark: bool) -> Color {
        let (r, g, b) = match (self, dark) {
            (Self::Cyan, true) => (0.220, 0.741, 0.973),  // #38BDF8
            (Self::Cyan, false) => (0.014, 0.412, 0.635), // #0369A1
            (Self::Magenta, true) => (0.957, 0.447, 0.714), // #F472B6
            (Self::Magenta, false) => (0.745, 0.094, 0.365), // #BE185D
            (Self::Acid, true) => (0.290, 0.871, 0.502),  // #4ADE80
            (Self::Acid, false) => (0.082, 0.502, 0.239), // #15803D
            (Self::Violet, true) => (0.655, 0.545, 0.980), // #A78BFA
            (Self::Violet, false) => (0.486, 0.227, 0.929), // #7C3AED
            (Self::Ember, true) => (0.984, 0.573, 0.235), // #FB923C
            (Self::Ember, false) => (0.761, 0.255, 0.047), // #C2410C
            (Self::Volt, true) => (0.376, 0.647, 0.980),  // #60A5FA
            (Self::Volt, false) => (0.114, 0.306, 0.847), // #1D4ED8
            (Self::Solar, true) => (0.984, 0.749, 0.141), // #FBBF24
            (Self::Solar, false) => (0.631, 0.384, 0.027), // #A16207
        };
        Color::from_rgb(r, g, b)
    }

    /// Ink that the iced-cube ink rule places on this hue.
    #[must_use]
    pub const fn on(self, dark: bool) -> Color {
        if raw_luma(self.base(dark)) > 0.55 {
            Color::from_rgb(0.035, 0.035, 0.035)
        } else {
            Color::from_rgb(0.980, 0.980, 0.980)
        }
    }
}

/// Raw (non-linearised) luminance — the same measure `iced-cube` uses.
const fn raw_luma(color: Color) -> f32 {
    0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b
}

/// Mix two colors in RGB space.
pub(crate) const fn mix(from: Color, to: Color, amount: f32) -> Color {
    Color::from_rgb(
        from.r + (to.r - from.r) * amount,
        from.g + (to.g - from.g) * amount,
        from.b + (to.b - from.b) * amount,
    )
}

/// Kit default hue: [`Neon::Cyan`], which also styles the boot workspace.
const DARK_ACCENT: Color = Neon::Cyan.base(true);
const LIGHT_ACCENT: Color = Neon::Cyan.base(false);

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
    /// Studio dark: slate frame, graphite cards, high-contrast text and a
    /// restrained cyan default accent (the shell swaps [`Neon`] per workspace).
    pub const DARK: Self = Self {
        background: Color::from_rgb(0.043, 0.071, 0.125),
        glass: Color::from_rgb(0.067, 0.102, 0.169),
        glass_strong: Color::from_rgb(0.086, 0.133, 0.208),
        surface_raised: Color::from_rgb(0.114, 0.169, 0.247),
        text: Color::from_rgb(0.945, 0.961, 0.976),
        muted: Color::from_rgb(0.667, 0.722, 0.800),
        faint: Color::from_rgb(0.518, 0.592, 0.690),
        accent: DARK_ACCENT,
        accent_text: Neon::Cyan.on(true),
        accent_hover: mix(DARK_ACCENT, Color::WHITE, 0.08),
        accent_pressed: mix(DARK_ACCENT, Color::BLACK, 0.18),
        accent_secondary: Color::from_rgb(0.667, 0.722, 0.800),
        border: Color::from_rgba(0.75, 0.82, 0.92, 0.12),
        border_strong: Color {
            a: 0.62,
            ..DARK_ACCENT
        },
        shadow: Color::from_rgba(0.0, 0.0, 0.0, 0.40),
        scrim: Color::from_rgba(0.02, 0.04, 0.08, 0.74),
        success: Color::from_rgb(0.404, 0.827, 0.573),
        warning: Color::from_rgb(0.937, 0.769, 0.353),
        danger: Color::from_rgb(0.957, 0.478, 0.443),
        wave: DARK_ACCENT,
        wave_hot: mix(DARK_ACCENT, Color::WHITE, 0.32),
        grid: Color::from_rgba(0.75, 0.82, 0.92, 0.08),
        playhead: Color::from_rgb(0.945, 0.961, 0.976),
    };

    /// Studio light: paper frame and cards, slate ink and a deepened default
    /// accent (the shell swaps [`Neon`] per workspace).
    pub const LIGHT: Self = Self {
        background: Color::from_rgb(0.945, 0.961, 0.976),
        glass: Color::from_rgb(0.910, 0.933, 0.957),
        glass_strong: Color::from_rgb(1.0, 1.0, 1.0),
        surface_raised: Color::from_rgb(0.973, 0.980, 0.988),
        text: Color::from_rgb(0.059, 0.090, 0.165),
        muted: Color::from_rgb(0.278, 0.337, 0.416),
        faint: Color::from_rgb(0.353, 0.420, 0.502),
        accent: LIGHT_ACCENT,
        accent_text: Neon::Cyan.on(false),
        accent_hover: mix(LIGHT_ACCENT, Color::WHITE, 0.08),
        accent_pressed: mix(LIGHT_ACCENT, Color::BLACK, 0.18),
        accent_secondary: Color::from_rgb(0.278, 0.337, 0.416),
        border: Color::from_rgba(0.10, 0.16, 0.25, 0.14),
        border_strong: Color {
            a: 0.70,
            ..LIGHT_ACCENT
        },
        shadow: Color::from_rgba(0.02, 0.04, 0.08, 0.12),
        scrim: Color::from_rgba(0.02, 0.04, 0.08, 0.48),
        success: Color::from_rgb(0.129, 0.510, 0.290),
        warning: Color::from_rgb(0.663, 0.467, 0.114),
        danger: Color::from_rgb(0.741, 0.243, 0.212),
        wave: LIGHT_ACCENT,
        wave_hot: mix(LIGHT_ACCENT, Color::WHITE, 0.18),
        grid: Color::from_rgba(0.10, 0.16, 0.25, 0.09),
        playhead: Color::from_rgb(0.059, 0.090, 0.165),
    };

    /// Resolve a theme mode, including the desktop preference.
    #[must_use]
    pub fn for_mode(mode: ThemeMode) -> Self {
        match mode.resolve(crate::tokens::system_is_dark()) {
            ThemeMode::Light => Self::LIGHT,
            _ => Self::DARK,
        }
    }

    /// Swap the accent family onto a workspace [`Neon`] hue.
    ///
    /// The base surfaces stay slate/paper; the hue takes over the accent
    /// (fills, focus borders, waveform body and highlight) together with its
    /// derived hover, pressed, hot and ink variants. The shell calls this once
    /// per view with the active workspace's hue, so buttons, sliders, toggles
    /// and every iced-cube control follow the workspace without per-widget
    /// styling.
    #[must_use]
    pub fn with_neon(self, neon: Neon) -> Self {
        let dark = raw_luma(self.background) <= 0.5;
        let base = neon.base(dark);
        Self {
            accent: base,
            accent_text: neon.on(dark),
            accent_hover: mix(base, Color::WHITE, 0.08),
            accent_pressed: mix(base, Color::BLACK, 0.18),
            border_strong: Color {
                a: if dark { 0.62 } else { 0.70 },
                ..base
            },
            wave: base,
            wave_hot: if dark {
                mix(base, Color::WHITE, 0.32)
            } else {
                mix(base, Color::WHITE, 0.18)
            },
            ..self
        }
    }

    /// iced theme handed to built-in widgets and to `iced-cube` controls.
    ///
    /// The palette is built through `iced_cube::theme::Config` so every
    /// iced-cube component resolves the same tokens as the kit: the neon
    /// accent becomes the cube `primary`, raised surfaces become its
    /// secondary and hover accents, and the studio border hairline becomes
    /// its edge colour. Cube picks the ink on primary fills with its own
    /// luminance rule, which the [`Neon`] bases are calibrated for.
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
                color: self.border,
            },
            shadow: Shadow {
                color: self.shadow,
                offset: Vector::new(0.0, 12.0),
                blur_radius: 32.0,
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

    #[test]
    fn neon_hues_are_all_distinct_per_mode() {
        for dark in [true, false] {
            for (i, a) in Neon::ALL.into_iter().enumerate() {
                for b in Neon::ALL.into_iter().skip(i + 1) {
                    assert_ne!(a.base(dark), b.base(dark), "{a:?} vs {b:?} (dark={dark})");
                }
            }
        }
    }

    #[test]
    fn with_neon_is_idempotent_for_the_default_hue() {
        let dark = ThemeColors::DARK;
        assert_eq!(dark.with_neon(Neon::Cyan), dark);
        let light = ThemeColors::LIGHT;
        assert_eq!(light.with_neon(Neon::Cyan), light);
        assert_ne!(dark.with_neon(Neon::Magenta), dark);
        assert_ne!(light.with_neon(Neon::Violet), light);
    }

    #[test]
    fn derived_accent_variants_stay_ordered() {
        for hue in Neon::ALL {
            for dark in [true, false] {
                let colors = ThemeColors::DARK.with_neon(hue);
                assert_ne!(colors.accent_hover, colors.accent, "{hue:?} (dark={dark})");
                assert_ne!(
                    colors.accent_pressed, colors.accent,
                    "{hue:?} (dark={dark})"
                );
                assert_ne!(colors.wave_hot, colors.wave, "{hue:?} (dark={dark})");
                assert!(raw_luma(colors.accent_hover) >= raw_luma(colors.accent));
                assert!(raw_luma(colors.accent_pressed) <= raw_luma(colors.accent));
            }
        }
    }

    #[test]
    fn with_neon_puts_cube_ink_on_the_base() {
        for hue in Neon::ALL {
            for palette in [ThemeColors::DARK, ThemeColors::LIGHT] {
                let dark = raw_luma(palette.background) <= 0.5;
                let colors = palette.with_neon(hue);
                assert_eq!(colors.accent_text, hue.on(dark), "{hue:?} (dark={dark})");
            }
        }
    }

    fn linear(channel: f32) -> f32 {
        if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    }

    fn luminance(color: Color) -> f32 {
        0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
    }

    fn contrast(foreground: Color, background: Color) -> f32 {
        let (light, dark) = match (luminance(foreground), luminance(background)) {
            (a, b) if a >= b => (a, b),
            (a, b) => (b, a),
        };
        (light + 0.05) / (dark + 0.05)
    }

    #[test]
    fn every_text_role_passes_wcag_aa_on_every_surface() {
        for palette in [ThemeColors::DARK, ThemeColors::LIGHT] {
            for surface in [palette.glass, palette.glass_strong, palette.surface_raised] {
                for (role, color) in [
                    ("text", palette.text),
                    ("muted", palette.muted),
                    ("faint", palette.faint),
                ] {
                    let ratio = contrast(color, surface);
                    assert!(ratio >= 4.5, "{role} on surface: {ratio:.2}:1 needs 4.5:1");
                }
            }
        }
    }

    #[test]
    fn every_accent_ink_passes_wcag_aa() {
        for hue in Neon::ALL {
            for dark in [true, false] {
                let base = hue.base(dark);
                let ratio = contrast(hue.on(dark), base);
                assert!(
                    ratio >= 4.5,
                    "{hue:?} ink on base: {ratio:.2}:1 needs 4.5:1 (dark={dark})"
                );
            }
        }
    }
}
