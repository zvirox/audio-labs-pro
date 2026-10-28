//! Floating surfaces: cards, modals, toasts and the command palette.
//!
//! These are pure builders. The application decides when a layer is shown and
//! owns the palette query/selection state, which keeps keyboard handling in one
//! place.

use iced::widget::{column, container, row, text, text_input};
use iced::{Alignment, Element, Length};

use crate::components::{self, ButtonKind};
use crate::fonts;
use crate::theme::ThemeColors;
use crate::tokens::{Radius, Spacing, TypeScale};

/// One executable palette row.
#[derive(Debug, Clone)]
pub struct CommandItem<Message> {
    /// Human-readable command name.
    pub title: String,
    /// Category or shortcut hint.
    pub hint: String,
    /// Message sent when the row is chosen.
    pub message: Message,
}

/// A raised glass card.
pub fn card<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    colors: ThemeColors,
) -> Element<'a, Message> {
    container(
        container(content)
            .padding(Spacing::MD)
            .style(move |_theme| colors.card_style()),
    )
    .padding(Spacing::XXS)
    .into()
}

/// A centered modal dialog with an action row.
pub fn modal<'a, Message: Clone + 'a>(
    title: impl Into<String>,
    body: impl Into<Element<'a, Message>>,
    actions: Vec<(String, Message)>,
    colors: ThemeColors,
) -> Element<'a, Message> {
    let mut buttons = row![]
        .spacing(f32::from(Spacing::SM))
        .align_y(Alignment::Center);
    for (label, message) in actions {
        buttons = buttons.push(components::button(
            label,
            ButtonKind::Primary,
            colors,
            Some(message),
        ));
    }

    let dialog = container(
        column![
            text(title.into())
                .size(f32::from(TypeScale::TITLE))
                .font(fonts::semibold())
                .color(colors.text),
            body.into(),
            buttons,
        ]
        .spacing(f32::from(Spacing::MD)),
    )
    .padding(Spacing::XL)
    .width(Length::Fixed(560.0))
    .style(move |_theme| colors.panel_style(true));

    container(dialog)
        .padding(Spacing::XXL)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(move |_theme| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgba(
                0.0, 0.0, 0.0, 0.55,
            ))),
            ..container::Style::default()
        })
        .into()
}

/// A transient toast at the bottom of the window.
pub fn toast<'a, Message: Clone + 'a>(
    message: impl Into<String>,
    colors: ThemeColors,
    on_dismiss: Message,
) -> Element<'a, Message> {
    let toast = row![
        text(message.into())
            .size(f32::from(TypeScale::BODY))
            .font(fonts::regular())
            .color(colors.text),
        components::button("Dismiss", ButtonKind::Ghost, colors, Some(on_dismiss)),
    ]
    .spacing(f32::from(Spacing::MD))
    .align_y(Alignment::Center);

    container(toast)
        .padding([Spacing::SM, Spacing::MD])
        .style(move |_theme| container::Style {
            background: Some(iced::Background::Color(colors.glass_strong)),
            border: iced::Border {
                radius: f32::from(Radius::LG).into(),
                width: 1.0,
                color: colors.border,
            },
            shadow: iced::Shadow {
                color: colors.shadow,
                offset: iced::Vector::new(0.0, 10.0),
                blur_radius: 30.0,
            },
            text_color: Some(colors.text),
            snap: true,
        })
        .into()
}

/// A modal command palette. The caller provides already-filtered items.
pub fn command_palette<'a, Message: Clone + 'a>(
    id: iced::widget::Id,
    query: &str,
    items: &[CommandItem<Message>],
    selected: usize,
    colors: ThemeColors,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    let mut list = column![].spacing(f32::from(Spacing::XXS));
    for (index, item) in items.iter().enumerate() {
        let label = format!("{}   {}", item.title, item.hint);
        let kind = if index == selected {
            ButtonKind::Secondary
        } else {
            ButtonKind::Ghost
        };
        list = list.push(components::button(
            label,
            kind,
            colors,
            Some(item.message.clone()),
        ));
    }

    let panel = column![
        text_input("Type a command…", query)
            .id(id)
            .on_input(on_input)
            .padding([Spacing::SM, Spacing::MD])
            .size(f32::from(TypeScale::BODY_STRONG))
            .font(fonts::regular()),
        list,
        components::muted("↑↓ move · Enter run · Esc close", colors),
    ]
    .spacing(f32::from(Spacing::SM))
    .padding(Spacing::MD)
    .width(Length::Fixed(640.0));

    container(panel)
        .padding(Spacing::XXL)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Start)
        .style(move |_theme| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgba(
                0.0, 0.0, 0.0, 0.32,
            ))),
            ..container::Style::default()
        })
        .into()
}

/// A labelled property row used by inspector-style panels.
pub fn property_row<'a, Message: 'a>(
    label: impl Into<String>,
    control: impl Into<Element<'a, Message>>,
    colors: ThemeColors,
) -> Element<'a, Message> {
    row![
        text(label.into())
            .size(f32::from(TypeScale::CAPTION))
            .font(fonts::regular())
            .color(colors.muted)
            .width(Length::Fixed(112.0)),
        control.into(),
    ]
    .spacing(f32::from(Spacing::SM))
    .align_y(Alignment::Center)
    .into()
}

/// Empty-state body for panels with no data.
pub fn empty_state<'a, Message: 'a>(
    title: impl Into<String>,
    hint: impl Into<String>,
    colors: ThemeColors,
) -> Element<'a, Message> {
    container(
        column![
            text(title.into())
                .size(f32::from(TypeScale::BODY_STRONG))
                .font(fonts::semibold())
                .color(colors.text),
            components::muted(hint, colors),
        ]
        .spacing(f32::from(Spacing::XS))
        .align_x(Alignment::Center),
    )
    .padding(Spacing::XL)
    .width(Length::Fill)
    .align_x(Alignment::Center)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    #[derive(Clone, Debug)]
    enum Msg {
        Input(String),
        Run(usize),
        Dismiss,
    }

    #[test]
    fn palette_builds_rows_and_input() {
        let items = vec![
            CommandItem {
                title: "Open Media Bin".into(),
                hint: "Ctrl+M".into(),
                message: Msg::Run(0),
            },
            CommandItem {
                title: "Toggle theme".into(),
                hint: "Ctrl+Shift+D".into(),
                message: Msg::Run(1),
            },
        ];
        let _ = command_palette(
            iced::widget::Id::unique(),
            "open",
            &items,
            0,
            ThemeColors::DARK,
            Msg::Input,
        );
    }

    #[test]
    fn modal_and_toast_build() {
        let _ = modal(
            "Reset layout?",
            components::muted("All panel positions will be restored.", ThemeColors::DARK),
            vec![("Reset".into(), Msg::Dismiss)],
            ThemeColors::DARK,
        );
        let _ = toast("Layout saved", ThemeColors::DARK, Msg::Dismiss);
    }
}
