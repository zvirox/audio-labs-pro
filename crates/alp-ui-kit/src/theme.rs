//! Dark and light studio palettes.
//!
//! Every screen reads colors from [`ThemeColors`]; no widget owns a literal
//! color. The dark palette is the studio default: near-black surfaces,
//! hairline white borders and a cyberpunk [`Neon`] accent that the shell swaps
//! per workspace. The light palette mirrors it with paper surfaces and a
//! deepened neon set, so one component set serves both.

use iced::widget::container;
use iced::{Background, Border, Color, Shadow, Theme, Vector};

use crate::tokens::{Radius, ThemeMode};

/// Cyberpunk neon accent hues, one per shell workspace.
///
/// The kit stays workspace-agnostic: hues are named for the color itself and
/// the shell maps each workspace onto one. Bases come in two calibrated sets —
/// vivid neon on the dark studio surfaces and a deepened set for paper light —
/// and every derived color ([`ThemeColors::accent_hover`], waveform hot, ink)
/// is computed from the base, so a hue can never drift out of sync.
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
            (Self::Cyan, true) => (0.0, 0.898, 1.0),         // #00E5FF
            (Self::Cyan, false) => (0.055, 0.455, 0.565),    // #0E7490
            (Self::Magenta, true) => (1.0, 0.180, 0.592),    // #FF2E97
            (Self::Magenta, false) => (0.784, 0.118, 0.471), // #C81E78
            (Self::Acid, true) => (0.357, 1.0, 0.302),       // #5BFF4D
            (Self::Acid, false) => (0.082, 0.502, 0.239),    // #15803D
            (Self::Violet, true) => (0.690, 0.149, 1.0),     // #B026FF
            (Self::Violet, false) => (0.486, 0.227, 0.929),  // #7C3AED
            (Self::Ember, true) => (1.0, 0.478, 0.161),      // #FF7A29
            (Self::Ember, false) => (0.761, 0.255, 0.047),   // #C2410C
            (Self::Volt, true) => (0.302, 0.702, 1.0),       // #4DB3FF
            (Self::Volt, false) => (0.145, 0.388, 0.922),    // #2563EB
            (Self::Solar, true) => (0.988, 0.933, 0.039),    // #FCEE0A
            (Self::Solar, false) => (0.792, 0.541, 0.016),   // #CA8A04
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
const fn mix(from: Color, to: Color, amount: f32) -> Color {
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
    /// Studio dark: near-black frame, graphite cards, electric cyan accent
    /// (the kit default hue — the shell swaps [`Neon`] per workspace).
    pub const DARK: Self = Self {
        background: Color::from_rgb(0.039, 0.039, 0.039),
        glass: Color::from_rgb(0.055, 0.055, 0.055),
        glass_strong: Color::from_rgb(0.078, 0.078, 0.078),
        surface_raised: Color::from_rgb(0.106, 0.106, 0.106),
        text: Color::from_rgb(0.960, 0.960, 0.960),
        muted: Color::from_rgb(0.620, 0.620, 0.620),
        faint: Color::from_rgb(0.400, 0.400, 0.400),
        accent: DARK_ACCENT,
        accent_text: Neon::Cyan.on(true),
        accent_hover: mix(DARK_ACCENT, Color::WHITE, 0.10),
        accent_pressed: mix(DARK_ACCENT, Color::BLACK, 0.15),
        accent_secondary: Color::from_rgb(0.655, 0.647, 0.620),
        border: Color::from_rgba(1.0, 1.0, 1.0, 0.10),
        border_strong: Color {
            a: 0.55,
            ..DARK_ACCENT
        },
        shadow: Color::from_rgba(0.0, 0.0, 0.0, 0.50),
        scrim: Color::from_rgba(0.0, 0.0, 0.0, 0.72),
        success: Color::from_rgb(0.478, 0.769, 0.541),
        warning: Color::from_rgb(0.878, 0.698, 0.361),
        danger: Color::from_rgb(0.878, 0.424, 0.373),
        wave: DARK_ACCENT,
        wave_hot: mix(DARK_ACCENT, Color::WHITE, 0.38),
        grid: Color::from_rgba(1.0, 1.0, 1.0, 0.055),
        playhead: Color::from_rgb(0.960, 0.960, 0.960),
    };

    /// Studio light: paper surfaces, graphite ink and the deepened default hue.
    pub const LIGHT: Self = Self {
        background: Color::from_rgb(0.910, 0.910, 0.910),
        glass: Color::from_rgb(0.898, 0.898, 0.898),
        glass_strong: Color::from_rgb(0.969, 0.969, 0.969),
        surface_raised: Color::from_rgb(1.0, 1.0, 1.0),
        text: Color::from_rgb(0.078, 0.078, 0.078),
        muted: Color::from_rgb(0.430, 0.430, 0.430),
        faint: Color::from_rgb(0.600, 0.600, 0.600),
        accent: LIGHT_ACCENT,
        accent_text: Neon::Cyan.on(false),
        accent_hover: mix(LIGHT_ACCENT, Color::WHITE, 0.10),
        accent_pressed: mix(LIGHT_ACCENT, Color::BLACK, 0.15),
        accent_secondary: Color::from_rgb(0.380, 0.380, 0.380),
        border: Color::from_rgba(0.0, 0.0, 0.0, 0.12),
        border_strong: Color {
            a: 0.65,
            ..LIGHT_ACCENT
        },
        shadow: Color::from_rgba(0.0, 0.0, 0.0, 0.16),
        scrim: Color::from_rgba(0.0, 0.0, 0.0, 0.45),
        success: Color::from_rgb(0.208, 0.518, 0.290),
        warning: Color::from_rgb(0.663, 0.467, 0.114),
        danger: Color::from_rgb(0.741, 0.243, 0.212),
        wave: LIGHT_ACCENT,
        wave_hot: mix(LIGHT_ACCENT, Color::WHITE, 0.38),
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

    /// Swap the accent family onto a cyberpunk [`Neon`] hue.
    ///
    /// The base surfaces stay graphite/paper; the hue takes over the accent
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
            accent_hover: mix(base, Color::WHITE, 0.10),
            accent_pressed: mix(base, Color::BLACK, 0.15),
            border_strong: Color {
                a: if dark { 0.55 } else { 0.65 },
                ..base
            },
            wave: base,
            wave_hot: mix(base, Color::WHITE, 0.38),
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
}
