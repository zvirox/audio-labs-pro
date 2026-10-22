//! Design-reference components for the Phase 0 spike. These are throwaway
//! proof-of-concept widgets; the real ones live in `alp-ui-kit` from Phase 2.

use iced::widget::{Space, button, container, row, text, text_input};
use iced::{Background, Border, Color, Element, Font, Length, Padding, Shadow, Vector, alignment};

use crate::Message;
use crate::glass;

fn ui_font() -> Font {
    Font {
        family: iced::font::Family::Name("Inter"),
        ..Font::DEFAULT
    }
}

fn semibold() -> Font {
    Font {
        weight: iced::font::Weight::Semibold,
        ..ui_font()
    }
}

pub fn tab_button<'a>(label: &'a str, active: bool, message: Message) -> Element<'a, Message> {
    button(text(label).size(12).font(ui_font()))
        .padding(Padding::from([6, 12]))
        .on_press(message)
        .style(move |_theme, status| {
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: Some(Background::Color(if active {
                    Color::from_rgba(1.0, 1.0, 1.0, 0.14)
                } else if hovered {
                    Color::from_rgba(1.0, 1.0, 1.0, 0.08)
                } else {
                    Color::TRANSPARENT
                })),
                text_color: if active {
                    Color::WHITE
                } else {
                    Color::from_rgba(1.0, 1.0, 1.0, 0.62)
                },
                border: Border {
                    radius: 8.0.into(),
                    width: 1.0,
                    color: if active {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.12)
                    } else {
                        Color::TRANSPARENT
                    },
                },
                shadow: Shadow::default(),
                snap: true,
            }
        })
        .into()
}

pub fn primary<'a>(label: &'a str, message: Message) -> Element<'a, Message> {
    button(text(label).size(12).font(semibold()))
        .padding(Padding::from([8, 14]))
        .on_press(message)
        .style(|_theme, status| {
            let (bg, shadow) = match status {
                button::Status::Hovered => (
                    Color::from_rgb(0.46, 0.60, 1.0),
                    Shadow {
                        color: Color::from_rgba(0.35, 0.48, 1.0, 0.55),
                        offset: Vector::new(0.0, 4.0),
                        blur_radius: 18.0,
                    },
                ),
                button::Status::Pressed => (Color::from_rgb(0.34, 0.47, 0.86), Shadow::default()),
                _ => (Color::from_rgb(0.40, 0.54, 0.95), Shadow::default()),
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: Color::WHITE,
                border: Border {
                    radius: 10.0.into(),
                    width: 1.0,
                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.16),
                },
                shadow,
                snap: true,
            }
        })
        .into()
}

pub fn ghost<'a>(label: &'a str, message: Message) -> Element<'a, Message> {
    button(text(label).size(12).font(ui_font()))
        .padding(Padding::from([8, 12]))
        .on_press(message)
        .style(|_theme, status| {
            let bg = match status {
                button::Status::Hovered => Color::from_rgba(1.0, 1.0, 1.0, 0.10),
                button::Status::Pressed => Color::from_rgba(1.0, 1.0, 1.0, 0.16),
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: Color::from_rgba(1.0, 1.0, 1.0, 0.80),
                border: Border {
                    radius: 10.0.into(),
                    width: 1.0,
                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.10),
                },
                shadow: Shadow::default(),
                snap: true,
            }
        })
        .into()
}

pub fn disabled<'a>(label: &'a str) -> Element<'a, Message> {
    button(text(label).size(12).font(ui_font()))
        .padding(Padding::from([8, 14]))
        .style(|_theme, _status| button::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.05))),
            text_color: Color::from_rgba(1.0, 1.0, 1.0, 0.35),
            border: Border {
                radius: 10.0.into(),
                width: 1.0,
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.06),
            },
            shadow: Shadow::default(),
            snap: true,
        })
        .into()
}

pub fn showcase(
    reduce_motion: bool,
    toast_on: bool,
    modal_on: bool,
    palette_on: bool,
) -> Element<'static, Message> {
    let _ = (toast_on, modal_on, palette_on);
    row![
        container(
            iced::widget::column![
                text("Buttons")
                    .size(12)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.5)),
                primary("Generate speech", Message::Toast),
                ghost("Secondary", Message::Toast),
                disabled("Loading…"),
                text(if reduce_motion {
                    "Reduce motion is on"
                } else {
                    "Motion tokens 120/200/320 ms"
                })
                .size(11)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.45)),
            ]
            .spacing(10),
        )
        .width(Length::FillPortion(1))
        .padding(16)
        .style(|_| glass::card_style()),
        container(
            iced::widget::column![
                text("Overlays")
                    .size(12)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.5)),
                primary("Toast", Message::Toast),
                ghost("Modal", Message::ToggleModal),
                ghost("Command palette", Message::TogglePalette),
            ]
            .spacing(10),
        )
        .width(Length::FillPortion(1))
        .padding(16)
        .style(|_| glass::card_style()),
    ]
    .spacing(14)
    .into()
}

pub fn toast(t: f32) -> Element<'static, Message> {
    let alpha = t.clamp(0.0, 1.0);
    container(
        row![
            text("✓").size(13).color(Color::from_rgb(0.55, 0.9, 0.7)),
            text("Export queued — 3 jobs").size(12).font(ui_font()),
            Space::new().width(Length::Fill),
            text("Esc")
                .size(10)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.4)),
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center),
    )
    .padding(14)
    .width(Length::Fixed(320.0))
    .style(move |_| {
        let mut style = glass::card_style();
        style.background = Some(Background::Color(Color::from_rgba(
            0.07,
            0.08,
            0.10,
            0.96 * alpha,
        )));
        style.text_color = Some(Color::from_rgba(1.0, 1.0, 1.0, alpha));
        style
    })
    .into()
}

pub fn modal(t: f32) -> Element<'static, Message> {
    let scrim = container(
        container(
            iced::widget::column![
                text("Confirm export").size(16).font(semibold()),
                text(
                    "Rendering 4 tracks to FLAC at 48 kHz / 24-bit. This runs in the \
                     worker pool and can be cancelled."
                )
                .size(12)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.65)),
                row![
                    primary("Start export", Message::ToggleModal),
                    ghost("Cancel", Message::ToggleModal),
                ]
                .spacing(10),
            ]
            .spacing(14),
        )
        .padding(22)
        .width(Length::Fixed(420.0))
        .style(|_| glass::card_style()),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .style(move |_| container::Style {
        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.5 * t))),
        ..container::Style::default()
    });

    scrim.into()
}

pub fn palette(t: f32) -> Element<'static, Message> {
    let alpha = t.clamp(0.0, 1.0);
    let items = [
        "Record to new track",
        "Normalize to -16 LUFS",
        "Generate voice from script",
        "Export stems",
        "Open model manager",
    ];

    let list = items
        .iter()
        .fold(iced::widget::column![].spacing(2), |column, item| {
            column.push(
                container(text(*item).size(12).font(ui_font()))
                    .padding(Padding::from([8, 10]))
                    .width(Length::Fill),
            )
        });

    container(
        container(
            iced::widget::column![
                text_input("Type a command…", "")
                    .padding(10)
                    .font(ui_font())
                    .size(13),
                list,
            ]
            .spacing(8),
        )
        .padding(12)
        .width(Length::Fixed(460.0))
        .style(|_| glass::card_style()),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(alignment::Horizontal::Center)
    .padding(Padding {
        top: 80.0 * alpha,
        ..Padding::ZERO
    })
    .into()
}
