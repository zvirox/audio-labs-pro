//! The Audio Lab Pro control set.
//!
//! Screens never construct raw iced buttons: every interactive control goes
//! through this module so variants, padding, focus and motion stay consistent.
//! Components are stateless; the application owns values and messages.

use iced::widget::{
    button as raw_button, button::Status as ButtonStatus, button::Style as ButtonStyle, pick_list,
    progress_bar, row, slider, text, toggler, tooltip,
};
use iced::{Background, Border, Color, Element, Length, Shadow};

use crate::fonts;
use crate::theme::ThemeColors;
use crate::tokens::{Control, Radius, Spacing, TypeScale};

/// Semantic button variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    /// Filled accent action.
    Primary,
    /// Outlined secondary action.
    Secondary,
    /// Borderless toolbar action.
    Ghost,
    /// Destructive action.
    Danger,
    /// Square glyph-only action.
    Icon,
}

/// A labelled button.
pub fn button<'a, Message: Clone + 'a>(
    label: impl Into<String>,
    kind: ButtonKind,
    colors: ThemeColors,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let label = label.into();
    raw_button(
        text(label)
            .size(f32::from(TypeScale::BODY))
            .font(fonts::medium()),
    )
    .padding(if kind == ButtonKind::Icon {
        [Spacing::SM, Spacing::SM]
    } else {
        [Spacing::SM, Spacing::MD]
    })
    .height(Length::Fixed(f32::from(Control::MD)))
    .style(move |_theme, status| button_style(kind, colors, status))
    .on_press_maybe(on_press)
    .into()
}

/// A compact glyph button; the glyph is text so it inherits Inter fallback.
pub fn icon_button<'a, Message: Clone + 'a>(
    glyph: impl Into<String>,
    colors: ThemeColors,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    button(glyph, ButtonKind::Icon, colors, on_press)
}

/// A labelled toggle switch.
pub fn toggle<'a, Message: Clone + 'a>(
    label: impl Into<String>,
    value: bool,
    colors: ThemeColors,
    on_toggle: Message,
) -> Element<'a, Message> {
    let label = label.into();
    toggler(value)
        .label(label)
        .size(36.0)
        .text_size(f32::from(TypeScale::BODY))
        .font(fonts::medium())
        .spacing(f32::from(Spacing::SM))
        .style(move |_theme, _status| toggler::Style {
            background: Background::Color(if value {
                colors.accent
            } else {
                colors.glass_strong
            }),
            background_border_width: 1.0,
            background_border_color: colors.border,
            foreground: Background::Color(colors.text),
            foreground_border_width: 0.0,
            foreground_border_color: Color::TRANSPARENT,
            text_color: Some(colors.text),
            border_radius: Some(f32::from(Radius::PILL).into()),
            padding_ratio: 0.18,
        })
        .on_toggle(move |_| on_toggle.clone())
        .into()
}

/// A horizontal slider in `0.0..=1.0`.
pub fn slider_control<'a, Message: Clone + 'a>(
    value: f32,
    on_change: impl Fn(f32) -> Message + 'a,
) -> Element<'a, Message> {
    slider(0.0..=1.0, value, on_change)
        .step(0.01_f32)
        .width(Length::Fill)
        .into()
}

/// A pick list with owned string options.
pub fn dropdown<'a, Message: Clone + 'a>(
    options: Vec<String>,
    selected: Option<String>,
    on_select: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    pick_list(options, selected, on_select)
        .text_size(f32::from(TypeScale::BODY))
        .font(fonts::regular())
        .padding([Spacing::XS, Spacing::SM])
        .width(Length::Fill)
        .into()
}

/// A tab strip. Active state is supplied by the caller.
pub fn tabs<'a, Message: Clone + 'a>(
    labels: &[&str],
    active: usize,
    colors: ThemeColors,
    on_select: impl Fn(usize) -> Message + 'a,
) -> Element<'a, Message> {
    let mut strip = row![].spacing(f32::from(Spacing::XS));
    for (index, label) in labels.iter().enumerate() {
        let kind = if index == active {
            ButtonKind::Secondary
        } else {
            ButtonKind::Ghost
        };
        strip = strip.push(button(*label, kind, colors, Some(on_select(index))));
    }
    strip.into()
}

/// A text tooltip around arbitrary content.
pub fn with_tooltip<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    label: impl Into<String>,
    colors: ThemeColors,
) -> Element<'a, Message> {
    tooltip(
        content,
        text(label.into())
            .size(f32::from(TypeScale::CAPTION))
            .font(fonts::regular())
            .color(colors.text),
        tooltip::Position::Top,
    )
    .gap(f32::from(Spacing::XS))
    .into()
}

/// A thin progress meter.
pub fn progress<'a, Message: 'a>(value: f32, colors: ThemeColors) -> Element<'a, Message> {
    progress_bar(0.0..=1.0, value)
        .length(Length::Fill)
        .girth(Length::Fixed(6.0))
        .style(move |_theme| progress_bar::Style {
            background: Background::Color(colors.glass_strong),
            bar: Background::Color(colors.accent),
            border: Border {
                radius: f32::from(Radius::PILL).into(),
                ..Border::default()
            },
        })
        .into()
}

/// Section heading.
pub fn section_title<'a, Message: 'a>(
    label: impl Into<String>,
    colors: ThemeColors,
) -> Element<'a, Message> {
    text(label.into())
        .size(f32::from(TypeScale::TITLE))
        .font(fonts::semibold())
        .color(colors.text)
        .into()
}

/// Secondary label.
pub fn muted<'a, Message: 'a>(
    label: impl Into<String>,
    colors: ThemeColors,
) -> Element<'a, Message> {
    text(label.into())
        .size(f32::from(TypeScale::CAPTION))
        .font(fonts::regular())
        .color(colors.muted)
        .into()
}

/// Small uppercase-style kicker.
pub fn kicker<'a, Message: 'a>(
    label: impl Into<String>,
    colors: ThemeColors,
) -> Element<'a, Message> {
    text(label.into().to_uppercase())
        .size(f32::from(TypeScale::CAPTION))
        .font(fonts::semibold())
        .color(colors.faint)
        .into()
}

fn button_style(kind: ButtonKind, colors: ThemeColors, status: ButtonStatus) -> ButtonStyle {
    let (background, text_color, border_color) = match kind {
        ButtonKind::Primary => match status {
            ButtonStatus::Hovered => (colors.accent_hover, colors.text, colors.accent_hover),
            ButtonStatus::Pressed => (colors.accent_pressed, colors.text, colors.accent_pressed),
            _ => (colors.accent, colors.text, colors.accent),
        },
        ButtonKind::Secondary => match status {
            ButtonStatus::Hovered | ButtonStatus::Pressed => {
                (colors.glass_strong, colors.text, colors.border_strong)
            }
            _ => (colors.glass, colors.text, colors.border),
        },
        ButtonKind::Ghost => match status {
            ButtonStatus::Hovered | ButtonStatus::Pressed => {
                (colors.glass_strong, colors.text, colors.border)
            }
            _ => (Color::TRANSPARENT, colors.muted, Color::TRANSPARENT),
        },
        ButtonKind::Danger => match status {
            ButtonStatus::Hovered | ButtonStatus::Pressed => {
                (colors.danger, colors.text, colors.danger)
            }
            _ => (colors.glass_strong, colors.danger, colors.danger),
        },
        ButtonKind::Icon => match status {
            ButtonStatus::Hovered | ButtonStatus::Pressed => {
                (colors.glass_strong, colors.text, colors.border_strong)
            }
            _ => (Color::TRANSPARENT, colors.muted, Color::TRANSPARENT),
        },
    };
    ButtonStyle {
        background: Some(Background::Color(background)),
        text_color,
        border: Border {
            radius: f32::from(Radius::SM).into(),
            width: 1.0,
            color: border_color,
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    #[derive(Clone, Debug)]
    enum Msg {
        Press,
        Toggle,
        Slide(f32),
    }

    #[test]
    fn buttons_build_with_and_without_messages() {
        let _ = button(
            "Play",
            ButtonKind::Primary,
            ThemeColors::DARK,
            Some(Msg::Press),
        );
        let _ = button::<Msg>("Disabled", ButtonKind::Ghost, ThemeColors::DARK, None);
    }

    #[test]
    fn toggle_and_slider_accept_messages() {
        let _ = toggle("Reduce motion", true, ThemeColors::DARK, Msg::Toggle);
        let _ = slider_control(0.5, Msg::Slide);
    }

    #[test]
    fn tabs_build_for_each_item() {
        let _ = tabs(&["A", "B", "C"], 1, ThemeColors::DARK, |index| {
            Msg::Slide(index as f32)
        });
    }
}
