//! Generate workspace: voice card grid, the generate box and the take history.

use alp_ui_kit::components;
use alp_ui_kit::icons;
use alp_ui_kit::tokens::{Spacing, TypeScale};
use alp_ui_kit::{ButtonKind, ThemeColors, fonts};
use iced::widget::{Space, column, container, row, text};
use iced::{Alignment, Element, Length};

use super::workspace_layout::header;
use super::{App, Message, format_time};

impl App {
    /// Generate workspace: pick a voice, render a take, review history.
    pub(super) fn generate_workspace(&self, colors: ThemeColors) -> Element<'_, Message> {
        let grid = self.profile_grid(3, colors);

        let left = column![
            grid,
            self.generate_box(colors),
            Space::new().height(Length::Fixed(0.0)),
        ]
        .spacing(f32::from(Spacing::LG))
        .width(Length::FillPortion(11));

        let right = column![self.history_panel(colors)]
            .width(Length::FillPortion(9))
            .height(Length::Fill);

        column![
            header(
                "Audio Labs Pro",
                "Local speech, cloned voices and every take on this machine",
                vec![
                    components::button_icon(
                        "Import Voice",
                        icons::ARROW_UP,
                        ButtonKind::Secondary,
                        Some(Message::ImportVoice)
                    ),
                    components::button_icon(
                        "Create Voice",
                        icons::GENERATE,
                        ButtonKind::Primary,
                        Some(Message::OpenCreateVoice)
                    ),
                ],
                colors,
            ),
            row![self.scroll(left, colors).height(Length::Fill), right]
                .spacing(f32::from(Spacing::XL))
                .width(Length::Fill)
                .height(Length::Fill),
        ]
        .spacing(f32::from(Spacing::LG))
        .height(Length::Fill)
        .into()
    }

    fn generate_box(&self, colors: ThemeColors) -> Element<'_, Message> {
        let profile_names: Vec<String> = self
            .profiles
            .iter()
            .map(|profile| profile.name.clone())
            .collect();
        let active_name = self
            .profile()
            .map_or_else(|| "No profile".to_owned(), |profile| profile.name.clone());

        container(
            column![
                components::section_title("Generate Speech", colors),
                components::field_label("Voice Profile", colors),
                components::dropdown(
                    profile_names,
                    Some(active_name),
                    colors,
                    Message::SelectVoice,
                ),
                components::field_label("Text to Speak", colors),
                components::text_area(
                    &self.voice_editor,
                    "Type what the voice should say…",
                    72.0,
                    colors,
                    Message::GenerateEditor,
                ),
                row![
                    components::button_large_icon(
                        if self.generating {
                            "Rendering…"
                        } else {
                            "Generate"
                        },
                        icons::GENERATE,
                        ButtonKind::Primary,
                        Some(Message::GenerateSpeech)
                    ),
                    components::button(
                        "Play last",
                        ButtonKind::Ghost,
                        Some(Message::PlayHistory(0))
                    ),
                    Space::new().width(Length::Fill),
                    components::muted(self.voice_status.clone(), colors),
                ]
                .spacing(f32::from(Spacing::SM))
                .align_y(Alignment::Center),
            ]
            .spacing(f32::from(Spacing::SM)),
        )
        .width(Length::Fill)
        .padding(Spacing::LG)
        .style(move |_theme| colors.card_style())
        .into()
    }

    fn history_panel(&self, colors: ThemeColors) -> Element<'_, Message> {
        let mut rows = column![].spacing(f32::from(Spacing::SM));
        if self.history.is_empty() {
            rows = rows.push(container(
                column![
                    text("No takes yet")
                        .size(f32::from(TypeScale::HEADING))
                        .font(fonts::semibold())
                        .color(colors.text),
                    components::muted(
                        "Write a line and press Generate. Every take renders locally and stays on this machine.",
                        colors,
                    ),
                ]
                .spacing(f32::from(Spacing::XS)),
            )
            .padding(Spacing::LG)
            .style(move |_theme| colors.well_style()));
        }
        for (index, item) in self.history.iter().enumerate() {
            rows = rows.push(self.history_row(index, item, colors));
        }

        let header = row![
            components::card_title("History", colors),
            Space::new().width(Length::Fill),
            components::muted(format!("{} take(s)", self.history.len()), colors),
            components::button("Clear", ButtonKind::Ghost, Some(Message::ClearHistory)),
        ]
        .spacing(f32::from(Spacing::XS))
        .align_y(Alignment::Center);

        column![header, self.scroll(rows, colors).height(Length::Fill),]
            .spacing(f32::from(Spacing::MD))
            .height(Length::Fill)
            .into()
    }

    fn history_row(
        &self,
        index: usize,
        item: &super::HistoryItem,
        colors: ThemeColors,
    ) -> Element<'_, Message> {
        let snippet = if item.text.chars().count() > 180 {
            let short: String = item.text.chars().take(180).collect();
            format!("{short}…")
        } else {
            item.text.clone()
        };

        container(
            row![
                icons::tinted(icons::WAVEFORM, 18.0, Some(colors.muted)),
                column![
                    text(item.voice_name.clone())
                        .size(f32::from(TypeScale::BODY_STRONG))
                        .font(fonts::semibold())
                        .color(colors.text),
                    row![
                        components::chip(item.kind, colors),
                        components::muted(format_time(item.duration_secs), colors),
                    ]
                    .spacing(f32::from(Spacing::XS))
                    .align_y(Alignment::Center),
                    components::muted(item.created.clone(), colors),
                ]
                .spacing(f32::from(Spacing::XXS))
                .width(Length::Fixed(132.0)),
                container(
                    text(snippet)
                        .size(f32::from(TypeScale::BODY))
                        .font(fonts::regular())
                        .color(colors.muted),
                )
                .width(Length::Fill)
                .padding(Spacing::MD)
                .style(move |_theme| colors.well_style()),
                column![
                    components::with_tooltip(
                        components::glyph_button(
                            icons::PLAY,
                            12.0,
                            ButtonKind::Ghost,
                            Some(Message::PlayHistory(index))
                        ),
                        "Play take".to_string(),
                        colors,
                    ),
                    components::with_tooltip(
                        components::glyph_button(
                            icons::PLUS,
                            13.0,
                            ButtonKind::Ghost,
                            Some(Message::InsertHistory(index))
                        ),
                        "Insert into timeline".to_string(),
                        colors,
                    ),
                    components::with_tooltip(
                        components::glyph_button(
                            icons::DELETE_KEY,
                            12.0,
                            ButtonKind::Ghost,
                            Some(Message::DeleteHistory(index))
                        ),
                        "Delete take".to_string(),
                        colors,
                    ),
                ]
                .spacing(f32::from(Spacing::XXS))
                .align_x(Alignment::End),
            ]
            .spacing(f32::from(Spacing::MD))
            .align_y(Alignment::Center)
            .padding(Spacing::MD),
        )
        .width(Length::Fill)
        .style(move |_theme| colors.card_style())
        .into()
    }
}
