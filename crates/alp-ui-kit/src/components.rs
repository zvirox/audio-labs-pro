//! The Audio Labs Pro control set.
//!
//! Screens never construct raw iced buttons: every interactive control goes
//! through this module so variants, padding, focus and motion stay consistent.
//! Components are stateless; the application owns values and messages.
//!
//! The controls are powered by [`iced-cube`](https://crates.io/crates/iced-cube):
//! buttons, switches, checkboxes and Lucide icons resolve their colours from
//! the active [`iced::Theme`], which [`ThemeColors::to_theme`] builds from the
//! studio palette. Token-styled surfaces such as chips, inputs, sliders and
//! scroll rails still take an explicit [`ThemeColors`] so they can draw on any
//! panel.
//!
//! The language is a studio one: pill buttons, circular rail glyphs, graphite
//! cards with hairline borders and one warm gold accent. Text helpers expose
//! the type scale so views never pick their own sizes.

use iced::widget::{
    button as raw_button, button::Status as ButtonStatus, button::Style as ButtonStyle, pick_list,
    progress_bar, row, slider, text, text_editor, text_input, tooltip,
};
use iced::{Background, Border, Color, Element, Length, Shadow};
use iced_cube::primitives::button as cube_button;
use iced_cube::primitives::checkbox as cube_checkbox;
use iced_cube::primitives::switch as cube_switch;
use iced_cube::theme::Tokens as CubeTokens;

use crate::fonts;
use crate::icons::{self, Glyph};
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

impl ButtonKind {
    /// The matching iced-cube variant used for colours.
    #[must_use]
    pub fn cube_variant(self) -> cube_button::Variant {
        match self {
            Self::Primary => cube_button::Variant::Primary,
            Self::Secondary => cube_button::Variant::Outline,
            Self::Ghost | Self::Icon => cube_button::Variant::Ghost,
            Self::Danger => cube_button::Variant::Destructive,
        }
    }
}

/// A labelled pill button.
pub fn button<'a, Message: Clone + 'a>(
    label: impl Into<String>,
    kind: ButtonKind,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    sized_button(label.into(), None, kind, on_press, None, Control::MD)
}

/// A labelled pill button that fills its parent width.
pub fn button_fill<'a, Message: Clone + 'a>(
    label: impl Into<String>,
    kind: ButtonKind,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    sized_button(
        label.into(),
        None,
        kind,
        on_press,
        Some(Length::Fill),
        Control::MD,
    )
}

/// A large pill button with a leading Lucide icon, for page-level actions.
pub fn button_large_icon<'a, Message: Clone + 'a>(
    label: impl Into<String>,
    icon: Glyph,
    kind: ButtonKind,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    sized_button(label.into(), Some(icon), kind, on_press, None, Control::LG)
}

/// A large pill button for page-level actions.
pub fn button_large<'a, Message: Clone + 'a>(
    label: impl Into<String>,
    kind: ButtonKind,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    sized_button(label.into(), None, kind, on_press, None, Control::LG)
}

/// A labelled pill button with a leading Lucide icon.
pub fn button_icon<'a, Message: Clone + 'a>(
    label: impl Into<String>,
    icon: Glyph,
    kind: ButtonKind,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    sized_button(label.into(), Some(icon), kind, on_press, None, Control::MD)
}

fn sized_button<'a, Message: Clone + 'a>(
    label: String,
    icon: Option<Glyph>,
    kind: ButtonKind,
    on_press: Option<Message>,
    width: Option<Length>,
    height: u16,
) -> Element<'a, Message> {
    let styled_text = |value: String| {
        text(value)
            .size(f32::from(TypeScale::LABEL))
            .font(fonts::medium())
            .line_height(1.2)
    };
    let content: Element<'a, Message> = if let Some(icon) = icon {
        row![icon_view::<Message>(icon, 14.0, kind), styled_text(label)]
            .spacing(f32::from(Spacing::XS))
            .align_y(iced::Alignment::Center)
            .into()
    } else {
        styled_text(label).into()
    };
    let mut widget = raw_button(content)
        .padding([0.0, Spacing::LG as f32])
        .height(Length::Fixed(f32::from(height)))
        .style(move |theme, status| cube_style(kind, theme, status))
        .on_press_maybe(on_press);
    if let Some(width) = width {
        widget = widget.width(width);
    }
    widget.into()
}

/// A compact icon button for toolbars and card actions.
pub fn icon_button<'a, Message: Clone + 'a>(
    glyph: Glyph,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    glyph_button(glyph, 16.0, ButtonKind::Icon, on_press)
}

/// A circular icon button used by the navigation rail.
pub fn rail_button<'a, Message: Clone + 'a>(
    glyph: Glyph,
    active: bool,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let tint = move |theme: &iced::Theme| {
        let tokens = CubeTokens::of(theme);
        if active {
            tokens.primary
        } else {
            tokens.muted_foreground
        }
    };
    raw_button(icons::themed(glyph, 18.0, 1.0, tint))
        .width(Length::Fixed(f32::from(Rail::BUTTON)))
        .height(Length::Fixed(f32::from(Rail::BUTTON)))
        .padding(0.0)
        .style(move |_theme, status| rail_button_style(active, status))
        .on_press_maybe(on_press)
        .into()
}

/// A square icon button with a caller-chosen icon size.
pub fn glyph_button<'a, Message: Clone + 'a>(
    glyph: Glyph,
    size: f32,
    kind: ButtonKind,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    sized_glyph(glyph, size, kind, on_press, 38.0)
}

/// A compact icon button for dense cards, history rows and timelines.
pub fn glyph_button_compact<'a, Message: Clone + 'a>(
    glyph: Glyph,
    size: f32,
    kind: ButtonKind,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    sized_glyph(glyph, size, kind, on_press, 28.0)
}

fn sized_glyph<'a, Message: Clone + 'a>(
    glyph: Glyph,
    size: f32,
    kind: ButtonKind,
    on_press: Option<Message>,
    side: f32,
) -> Element<'a, Message> {
    raw_button(icon_view(glyph, size, kind))
        .padding(0.0)
        .width(Length::Fixed(side))
        .height(Length::Fixed(side))
        .style(move |theme, status| cube_style(kind, theme, status))
        .on_press_maybe(on_press)
        .into()
}

/// Draws a Lucide icon tinted with the rest colour of a button variant.
#[must_use]
pub fn icon_view<'a, Message: 'a>(
    glyph: Glyph,
    size: f32,
    kind: ButtonKind,
) -> Element<'a, Message> {
    icons::themed(glyph, size, 1.0, move |theme| {
        let tokens = CubeTokens::of(theme);
        match kind {
            ButtonKind::Primary => tokens.primary_foreground,
            ButtonKind::Danger => tokens.destructive_foreground,
            ButtonKind::Secondary => tokens.foreground,
            ButtonKind::Ghost | ButtonKind::Icon => tokens.muted_foreground,
        }
    })
    .into()
}

/// A labelled toggle switch.
pub fn toggle<'a, Message: Clone + 'a>(
    label: impl iced::widget::text::IntoFragment<'a>,
    value: bool,
    on_toggle: Message,
) -> Element<'a, Message> {
    cube_switch::switch(value)
        .label(label)
        .on_toggle(move |_| on_toggle.clone())
        .into()
}

/// A switch without a label, for dense rows that already describe themselves.
pub fn switch<'a, Message: Clone + 'a>(value: bool, on_toggle: Message) -> Element<'a, Message> {
    toggle(String::new(), value, on_toggle)
}

/// A labelled checkbox. Use it for confirmations and selectable options;
/// use [`switch`] for on/off settings.
///
/// `on_toggle` receives the new checked state.
pub fn checkbox<'a, Message: Clone + 'a>(
    label: impl iced::widget::text::IntoFragment<'a>,
    checked: bool,
    on_toggle: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    cube_checkbox::checkbox(checked)
        .label(label)
        .on_toggle(on_toggle)
        .into()
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
    on_select: impl Fn(usize) -> Message + 'a,
) -> Element<'a, Message> {
    let mut strip = row![].spacing(f32::from(Spacing::XS));
    for (index, label) in labels.iter().enumerate() {
        let kind = if index == active {
            ButtonKind::Secondary
        } else {
            ButtonKind::Ghost
        };
        strip = strip.push(button(*label, kind, Some(on_select(index))));
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

/// Button colours borrowed from iced-cube, with the studio pill radius.
fn cube_style(kind: ButtonKind, theme: &iced::Theme, status: ButtonStatus) -> ButtonStyle {
    let tokens = CubeTokens::of(theme);
    let mut style = cube_button::style(&tokens, kind.cube_variant(), status);
    style.border.radius = f32::from(Radius::PILL).into();
    style
}

fn rail_button_style(active: bool, status: ButtonStatus) -> ButtonStyle {
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
        text_color: Color::TRANSPARENT,
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
    use crate::icons;

    #[allow(dead_code)]
    #[derive(Clone, Debug)]
    enum Msg {
        Press,
        Toggle,
        Check,
        Slide(f32),
        Field(String),
        Segment(usize),
    }

    #[test]
    fn buttons_build_with_and_without_messages() {
        let _ = button("Play", ButtonKind::Primary, Some(Msg::Press));
        let _ = button::<Msg>("Disabled", ButtonKind::Ghost, None);
        let _: Element<'_, Msg> = button_fill("Full", ButtonKind::Secondary, None);
        let _: Element<'_, Msg> = button_large("Large", ButtonKind::Primary, None);
        let _: Element<'_, Msg> =
            button_icon("Create Voice", icons::GENERATE, ButtonKind::Primary, None);
    }

    #[test]
    fn rail_and_segment_build() {
        let _ = rail_button(icons::GENERATE, true, Some(Msg::Press));
        let _ = rail_button(icons::LIBRARY, false, Some(Msg::Press));
        let _ = segmented(&["Upload", "Record"], 1, ThemeColors::DARK, Msg::Segment);
    }

    #[test]
    fn icon_variants_build() {
        let _: Element<'_, Msg> = icon_button(icons::PLAY, Some(Msg::Press));
        let _: Element<'_, Msg> =
            glyph_button(icons::PLAY, 18.0, ButtonKind::Icon, Some(Msg::Press));
        let _: Element<'_, Msg> = glyph_button_compact(icons::TRIM, 14.0, ButtonKind::Ghost, None);
    }

    #[test]
    fn toggle_switch_and_checkbox_accept_messages() {
        let _ = toggle("Reduce motion", true, Msg::Toggle);
        let _: Element<'_, Msg> = switch(false, Msg::Toggle);
        let _: Element<'_, Msg> = checkbox("Export selection", false, |checked| {
            Msg::Slide(checked as u32 as f32)
        });
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
        let _ = tabs(&["A", "B", "C"], 1, |index| Msg::Slide(index as f32));
    }
}
