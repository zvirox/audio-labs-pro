//! The Audio Labs Pro control set.
//!
//! Screens never construct raw iced buttons: every interactive control goes
//! through this module so variants, padding, focus and motion stay consistent.
//! Components are stateless; the application owns values and messages.
//!
//! The language is a studio one: pill buttons, circular rail glyphs, graphite
//! cards with hairline borders and one warm gold accent. Text helpers expose the
//! type scale so views never pick their own sizes.

use iced::widget::{
    button as raw_button, button::Status as ButtonStatus, button::Style as ButtonStyle, pick_list,
    progress_bar, row, slider, text, text_editor, text_input, toggler, tooltip,
};
use iced::{Background, Border, Color, Element, Length, Shadow};

use crate::fonts;
use crate::theme::ThemeColors;
use crate::tokens::{Control, Radius, Rail, Spacing, TypeScale};

/// Semantic button variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    /// Filled gold action.
    Primary,
    /// Hairline outlined action.
    Secondary,
    /// Borderless toolbar action.
    Ghost,
    /// Destructive action.
    Danger,
    /// Square glyph-only action.
    Icon,
}

/// A labelled pill button.
pub fn button<'a, Message: Clone + 'a>(
    label: impl Into<String>,
    kind: ButtonKind,
    colors: ThemeColors,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    sized_button(label, kind, colors, on_press, None, Control::MD)
}

/// A labelled pill button that fills its parent width.
pub fn button_fill<'a, Message: Clone + 'a>(
    label: impl Into<String>,
    kind: ButtonKind,
    colors: ThemeColors,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    sized_button(
        label,
        kind,
        colors,
        on_press,
        Some(Length::Fill),
        Control::MD,
    )
}

/// A large pill button for page-level actions.
pub fn button_large<'a, Message: Clone + 'a>(
    label: impl Into<String>,
    kind: ButtonKind,
    colors: ThemeColors,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    sized_button(label, kind, colors, on_press, None, Control::LG)
}

fn sized_button<'a, Message: Clone + 'a>(
    label: impl Into<String>,
    kind: ButtonKind,
    colors: ThemeColors,
    on_press: Option<Message>,
    width: Option<Length>,
    height: u16,
) -> Element<'a, Message> {
    let label = label.into();
    let padding = if kind == ButtonKind::Icon {
        [0.0, Spacing::SM as f32]
    } else {
        [0.0, Spacing::LG as f32]
    };
    let mut widget = raw_button(
        text(label)
            .size(f32::from(TypeScale::LABEL))
            .font(fonts::medium())
            .line_height(1.2),
    )
    .padding(padding)
    .height(Length::Fixed(f32::from(height)))
    .style(move |_theme, status| button_style(kind, colors, status))
    .on_press_maybe(on_press);
    if let Some(width) = width {
        widget = widget.width(width);
    }
    widget.into()
}

/// A compact glyph button; the glyph is text so it inherits the bundled font.
pub fn icon_button<'a, Message: Clone + 'a>(
    glyph: impl Into<String>,
    colors: ThemeColors,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    button(glyph, ButtonKind::Icon, colors, on_press)
}

/// A circular glyph button used by the navigation rail.
pub fn rail_button<'a, Message: Clone + 'a>(
    glyph: impl Into<String>,
    active: bool,
    colors: ThemeColors,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let glyph = glyph.into();
    raw_button(
        text(glyph)
            .size(18.0)
            .font(fonts::regular())
            .color(if active { colors.accent } else { colors.muted }),
    )
    .width(Length::Fixed(f32::from(Rail::BUTTON)))
    .height(Length::Fixed(f32::from(Rail::BUTTON)))
    .padding(0.0)
    .style(move |_theme, status| rail_button_style(active, colors, status))
    .on_press_maybe(on_press)
    .into()
}

/// A square glyph button with a caller-chosen glyph size.
pub fn glyph_button<'a, Message: Clone + 'a>(
    glyph: impl Into<String>,
    size: f32,
    kind: ButtonKind,
    colors: ThemeColors,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    sized_glyph(glyph, size, kind, colors, on_press, 38.0)
}

/// A compact glyph button for dense cards, history rows and timelines.
pub fn glyph_button_compact<'a, Message: Clone + 'a>(
    glyph: impl Into<String>,
    size: f32,
    kind: ButtonKind,
    colors: ThemeColors,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    sized_glyph(glyph, size, kind, colors, on_press, 28.0)
}

fn sized_glyph<'a, Message: Clone + 'a>(
    glyph: impl Into<String>,
    size: f32,
    kind: ButtonKind,
    colors: ThemeColors,
    on_press: Option<Message>,
    side: f32,
) -> Element<'a, Message> {
    raw_button(text(glyph.into()).size(size).font(fonts::regular()))
        .padding(0.0)
        .width(Length::Fixed(side))
        .height(Length::Fixed(side))
        .style(move |_theme, status| button_style(kind, colors, status))
        .on_press_maybe(on_press)
        .into()
}

/// A labelled toggle switch.
pub fn toggle<'a, Message: Clone + 'a>(
    label: impl Into<String>,
    value: bool,
    colors: ThemeColors,
    on_toggle: Message,
) -> Element<'a, Message> {
    styled_toggler(label.into(), value, colors)
        .on_toggle(move |_| on_toggle.clone())
        .into()
}

/// A switch without a label, for dense rows that already describe themselves.
pub fn switch<'a, Message: Clone + 'a>(
    value: bool,
    colors: ThemeColors,
    on_toggle: Message,
) -> Element<'a, Message> {
    styled_toggler(String::new(), value, colors)
        .on_toggle(move |_| on_toggle.clone())
        .into()
}

fn styled_toggler<'a, Message: Clone + 'a>(
    label: String,
    value: bool,
    colors: ThemeColors,
) -> iced::widget::Toggler<'a, Message> {
    toggler(value)
        .label(label)
        .size(34.0)
        .text_size(f32::from(TypeScale::LABEL))
        .font(fonts::medium())
        .spacing(f32::from(Spacing::SM))
        .style(move |_theme, _status| toggler::Style {
            background: Background::Color(if value {
                colors.accent
            } else {
                colors.surface_raised
            }),
            background_border_width: 1.0,
            background_border_color: if value { colors.accent } else { colors.border },
            foreground: Background::Color(if value {
                colors.accent_text
            } else {
                colors.muted
            }),
            foreground_border_width: 0.0,
            foreground_border_color: Color::TRANSPARENT,
            text_color: Some(colors.text),
            border_radius: Some(f32::from(Radius::PILL).into()),
            padding_ratio: 0.16,
        })
}

/// A horizontal slider that reads the active palette.
pub fn themed_slider<'a, Message: Clone + 'a>(
    value: f32,
    colors: ThemeColors,
    on_change: impl Fn(f32) -> Message + 'a,
) -> Element<'a, Message> {
    slider(0.0..=1.0, value, on_change)
        .step(0.01_f32)
        .width(Length::Fill)
        .style(slider_style(colors))
        .into()
}

/// A legacy alias for [`themed_slider`] kept for older call sites.
pub fn slider_control<'a, Message: Clone + 'a>(
    value: f32,
    on_change: impl Fn(f32) -> Message + 'a,
) -> Element<'a, Message> {
    themed_slider(value, ThemeColors::DARK, on_change)
}

fn slider_style(colors: ThemeColors) -> impl Fn(&iced::Theme, slider::Status) -> slider::Style {
    move |_theme, _status| slider::Style {
        rail: slider::Rail {
            backgrounds: (
                Background::Color(colors.border),
                Background::Color(colors.accent),
            ),
            width: 4.0,
            border: Border {
                radius: f32::from(Radius::PILL).into(),
                ..Border::default()
            },
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle { radius: 7.0 },
            background: Background::Color(colors.text),
            border_width: 2.0,
            border_color: colors.accent,
        },
    }
}

/// A styled single-line text field.
pub fn text_field<'a, Message: Clone + 'a>(
    placeholder: &'a str,
    value: &'a str,
    colors: ThemeColors,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    text_input(placeholder, value)
        .on_input(on_input)
        .padding([Spacing::SM, Spacing::MD])
        .size(f32::from(TypeScale::BODY))
        .font(fonts::regular())
        .style(move |_theme, status| input_style(colors, status))
        .into()
}

/// A styled multi-line text area.
pub fn text_area<'a, Message: Clone + 'a>(
    content: &'a text_editor::Content,
    placeholder: &'a str,
    height: f32,
    colors: ThemeColors,
    on_action: impl Fn(text_editor::Action) -> Message + 'a,
) -> Element<'a, Message> {
    text_editor(content)
        .placeholder(placeholder)
        .on_action(on_action)
        .height(Length::Fixed(height))
        .padding(Spacing::MD)
        .size(f32::from(TypeScale::BODY))
        .font(fonts::regular())
        .style(move |_theme, status| editor_style(colors, status))
        .into()
}

/// A pick list styled like the rest of the fields.
pub fn dropdown<'a, Message: Clone + 'a>(
    options: Vec<String>,
    selected: Option<String>,
    colors: ThemeColors,
    on_select: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    pick_list(options, selected, on_select)
        .text_size(f32::from(TypeScale::BODY))
        .font(fonts::regular())
        .padding([Spacing::SM, Spacing::MD])
        .width(Length::Fill)
        .style(move |_theme, status| pick_list_style(colors, status))
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

/// A segmented control: one recessed track, one selected pill.
pub fn segmented<'a, Message: Clone + 'a>(
    labels: &[&'a str],
    active: usize,
    colors: ThemeColors,
    on_select: impl Fn(usize) -> Message + 'a,
) -> Element<'a, Message> {
    let mut strip = row![].spacing(f32::from(Spacing::XXS));
    for (index, label) in labels.iter().enumerate() {
        let selected = index == active;
        strip = strip.push(
            raw_button(
                text(*label)
                    .size(f32::from(TypeScale::LABEL))
                    .font(if selected {
                        fonts::semibold()
                    } else {
                        fonts::regular()
                    })
                    .color(if selected { colors.text } else { colors.muted }),
            )
            .width(Length::Fill)
            .height(Length::Fixed(38.0))
            .padding([0.0, Spacing::MD as f32])
            .style(move |_theme, status| segment_style(selected, colors, status))
            .on_press(on_select(index)),
        );
    }
    container_track(strip.into(), colors)
}

fn container_track<'a, Message: 'a>(
    content: Element<'a, Message>,
    colors: ThemeColors,
) -> Element<'a, Message> {
    iced::widget::container(content)
        .padding(f32::from(Spacing::XXS))
        .width(Length::Fill)
        .style(move |_theme| iced::widget::container::Style {
            background: Some(Background::Color(colors.surface_raised)),
            border: Border {
                radius: f32::from(Radius::MD).into(),
                width: 1.0,
                color: colors.border,
            },
            shadow: Shadow::default(),
            text_color: Some(colors.text),
            snap: true,
        })
        .into()
}

/// A small rounded language or status chip.
pub fn chip<'a, Message: 'a>(
    label: impl Into<String>,
    colors: ThemeColors,
) -> Element<'a, Message> {
    iced::widget::container(
        text(label.into())
            .size(f32::from(TypeScale::CAPTION))
            .font(fonts::medium())
            .color(colors.muted),
    )
    .padding([2.0, Spacing::SM as f32])
    .style(move |_theme| iced::widget::container::Style {
        background: Some(Background::Color(colors.surface_raised)),
        border: Border {
            radius: f32::from(Radius::SM).into(),
            width: 1.0,
            color: colors.border,
        },
        shadow: Shadow::default(),
        text_color: Some(colors.muted),
        snap: true,
    })
    .into()
}

/// A circular initials avatar.
pub fn avatar<'a, Message: 'a>(
    name: &str,
    diameter: f32,
    colors: ThemeColors,
) -> Element<'a, Message> {
    let initials: String = name
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase();
    let initials = if initials.is_empty() {
        "?".to_owned()
    } else {
        initials
    };
    iced::widget::container(
        text(initials)
            .size(diameter * 0.34)
            .font(fonts::semibold())
            .color(colors.accent),
    )
    .width(Length::Fixed(diameter))
    .height(Length::Fixed(diameter))
    .align_x(iced::Alignment::Center)
    .align_y(iced::alignment::Vertical::Center)
    .style(move |_theme| iced::widget::container::Style {
        background: Some(Background::Color(colors.surface_raised)),
        border: Border {
            radius: (diameter / 2.0).into(),
            width: 1.0,
            color: colors.border,
        },
        shadow: Shadow::default(),
        text_color: Some(colors.accent),
        snap: true,
    })
    .into()
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

/// Scrollbar and rail styling shared by every scrolling surface.
pub fn scroll_style(colors: ThemeColors) -> iced::widget::scrollable::Style {
    let rail = iced::widget::scrollable::Rail {
        background: None,
        border: Border::default(),
        scroller: iced::widget::scrollable::Scroller {
            background: Background::Color(Color {
                a: 0.18,
                ..colors.muted
            }),
            border: Border {
                radius: 3.0.into(),
                ..Border::default()
            },
        },
    };
    iced::widget::scrollable::Style {
        container: iced::widget::container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
        auto_scroll: iced::widget::scrollable::AutoScroll {
            background: Background::Color(colors.glass_strong),
            border: Border {
                radius: f32::from(Radius::SM).into(),
                width: 1.0,
                color: colors.border,
            },
            shadow: iced::Shadow::default(),
            icon: colors.muted,
        },
    }
}

/// A thin progress meter.
pub fn progress<'a, Message: 'a>(value: f32, colors: ThemeColors) -> Element<'a, Message> {
    progress_bar(0.0..=1.0, value)
        .length(Length::Fill)
        .girth(Length::Fixed(4.0))
        .style(move |_theme| progress_bar::Style {
            background: Background::Color(colors.surface_raised),
            bar: Background::Color(colors.accent),
            border: Border {
                radius: f32::from(Radius::PILL).into(),
                ..Border::default()
            },
        })
        .into()
}

/// Page title at the top of a workspace.
pub fn page_title<'a, Message: 'a>(
    label: impl Into<String>,
    colors: ThemeColors,
) -> Element<'a, Message> {
    text(label.into())
        .size(f32::from(TypeScale::DISPLAY))
        .font(fonts::bold())
        .color(colors.text)
        .into()
}

/// Section heading inside a card or dialog.
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

/// Card heading.
pub fn card_title<'a, Message: 'a>(
    label: impl Into<String>,
    colors: ThemeColors,
) -> Element<'a, Message> {
    text(label.into())
        .size(f32::from(TypeScale::HEADING))
        .font(fonts::semibold())
        .color(colors.text)
        .into()
}

/// Field label above an input.
pub fn field_label<'a, Message: 'a>(
    label: impl Into<String>,
    colors: ThemeColors,
) -> Element<'a, Message> {
    text(label.into())
        .size(f32::from(TypeScale::LABEL))
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
        .color(colors.accent_secondary)
        .into()
}

fn button_style(kind: ButtonKind, colors: ThemeColors, status: ButtonStatus) -> ButtonStyle {
    let hovered = matches!(status, ButtonStatus::Hovered | ButtonStatus::Pressed);
    let (background, text_color, border_color) = match kind {
        ButtonKind::Primary => match status {
            ButtonStatus::Hovered => (colors.accent_hover, colors.accent_text, colors.accent_hover),
            ButtonStatus::Pressed => (
                colors.accent_pressed,
                colors.accent_text,
                colors.accent_pressed,
            ),
            _ => (colors.accent, colors.accent_text, colors.accent),
        },
        ButtonKind::Secondary => (
            if hovered {
                colors.surface_raised
            } else {
                Color::TRANSPARENT
            },
            colors.text,
            if hovered {
                colors.border_strong
            } else {
                colors.border
            },
        ),
        ButtonKind::Ghost => (
            if hovered {
                colors.surface_raised
            } else {
                Color::TRANSPARENT
            },
            if hovered { colors.text } else { colors.muted },
            Color::TRANSPARENT,
        ),
        ButtonKind::Danger => (
            if hovered {
                colors.danger
            } else {
                Color::TRANSPARENT
            },
            if hovered { colors.text } else { colors.danger },
            colors.danger,
        ),
        ButtonKind::Icon => (
            if hovered {
                colors.surface_raised
            } else {
                Color::TRANSPARENT
            },
            if hovered { colors.text } else { colors.muted },
            Color::TRANSPARENT,
        ),
    };
    let (border_width, border_color) = if border_color == Color::TRANSPARENT {
        (0.0, Color::TRANSPARENT)
    } else {
        (1.0, border_color)
    };
    ButtonStyle {
        background: Some(Background::Color(background)),
        text_color,
        border: Border {
            radius: f32::from(Radius::PILL).into(),
            width: border_width,
            color: border_color,
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

fn rail_button_style(active: bool, colors: ThemeColors, status: ButtonStatus) -> ButtonStyle {
    let hovered = matches!(status, ButtonStatus::Hovered | ButtonStatus::Pressed);
    let background = if active {
        Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.07))
    } else if hovered {
        Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.05))
    } else {
        Background::Color(Color::TRANSPARENT)
    };
    ButtonStyle {
        background: Some(background),
        text_color: if active { colors.accent } else { colors.muted },
        border: Border {
            radius: f32::from(Radius::PILL).into(),
            width: if active { 1.0 } else { 0.0 },
            color: if active {
                Color::from_rgba(1.0, 1.0, 1.0, 0.08)
            } else {
                Color::TRANSPARENT
            },
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

fn segment_style(selected: bool, colors: ThemeColors, status: ButtonStatus) -> ButtonStyle {
    let hovered = matches!(status, ButtonStatus::Hovered | ButtonStatus::Pressed);
    ButtonStyle {
        background: Some(Background::Color(if selected || hovered {
            colors.glass_strong
        } else {
            Color::TRANSPARENT
        })),
        text_color: if selected { colors.text } else { colors.muted },
        border: Border {
            radius: f32::from(Radius::SM).into(),
            width: if selected { 1.0 } else { 0.0 },
            color: if selected {
                colors.border
            } else {
                Color::TRANSPARENT
            },
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

fn input_style(colors: ThemeColors, status: text_input::Status) -> text_input::Style {
    let focused = matches!(status, text_input::Status::Focused { .. });
    text_input::Style {
        background: Background::Color(colors.surface_raised),
        border: Border {
            radius: f32::from(Radius::SM).into(),
            width: 1.0,
            color: if focused {
                colors.border_strong
            } else {
                colors.border
            },
        },
        icon: colors.muted,
        placeholder: colors.faint,
        value: colors.text,
        selection: colors.border_strong,
    }
}

fn editor_style(colors: ThemeColors, status: text_editor::Status) -> text_editor::Style {
    let focused = matches!(status, text_editor::Status::Focused { .. });
    text_editor::Style {
        background: Background::Color(colors.surface_raised),
        border: Border {
            radius: f32::from(Radius::SM).into(),
            width: 1.0,
            color: if focused {
                colors.border_strong
            } else {
                colors.border
            },
        },
        placeholder: colors.faint,
        value: colors.text,
        selection: colors.border_strong,
    }
}

fn pick_list_style(colors: ThemeColors, status: pick_list::Status) -> pick_list::Style {
    let active = matches!(
        status,
        pick_list::Status::Hovered | pick_list::Status::Opened { .. }
    );
    pick_list::Style {
        text_color: colors.text,
        placeholder_color: colors.faint,
        handle_color: colors.muted,
        background: Background::Color(colors.surface_raised),
        border: Border {
            radius: f32::from(Radius::SM).into(),
            width: 1.0,
            color: if active {
                colors.border_strong
            } else {
                colors.border
            },
        },
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
        Field(String),
        Segment(usize),
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
        let _: Element<'_, Msg> =
            button_fill("Full", ButtonKind::Secondary, ThemeColors::DARK, None);
        let _: Element<'_, Msg> =
            button_large("Large", ButtonKind::Primary, ThemeColors::DARK, None);
    }

    #[test]
    fn rail_and_segment_build() {
        let _ = rail_button("✦", true, ThemeColors::DARK, Some(Msg::Press));
        let _ = rail_button("▤", false, ThemeColors::DARK, Some(Msg::Press));
        let _ = segmented(&["Upload", "Record"], 1, ThemeColors::DARK, Msg::Segment);
    }

    #[test]
    fn toggle_and_slider_accept_messages() {
        let _ = toggle("Reduce motion", true, ThemeColors::DARK, Msg::Toggle);
        let _ = slider_control(0.5, Msg::Slide);
        let _ = themed_slider(0.5, ThemeColors::DARK, Msg::Slide);
    }

    #[test]
    fn fields_and_text_helpers_build() {
        let _ = text_field("Name", "", ThemeColors::DARK, Msg::Field);
        let _: Element<'_, Msg> = chip("en", ThemeColors::DARK);
        let _: Element<'_, Msg> = avatar("Linus Sebastian", 40.0, ThemeColors::DARK);
        let _ = page_title::<Msg>("Generate", ThemeColors::DARK);
        let _ = section_title::<Msg>("Generate Speech", ThemeColors::DARK);
        let _ = card_title::<Msg>("Haden", ThemeColors::DARK);
        let _ = field_label::<Msg>("Voice Profile", ThemeColors::DARK);
    }

    #[test]
    fn tabs_build_for_each_item() {
        let _ = tabs(&["A", "B", "C"], 1, ThemeColors::DARK, |index| {
            Msg::Slide(index as f32)
        });
    }
}
