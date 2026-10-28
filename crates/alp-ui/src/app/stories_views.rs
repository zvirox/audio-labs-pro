//! Stories workspace: story list, line editor and the shared track editor.

use alp_ui_kit::components;
use alp_ui_kit::tokens::{Radius, Spacing, TypeScale};
use alp_ui_kit::{ButtonKind, ThemeColors, fonts};
use iced::widget::{Space, column, container, mouse_area, row, stack, text};
use iced::{Alignment, Background, Border, Element, Length};

use super::workspace_layout::{card, header};
use super::{App, Message};

fn status_label(status: alp_voice::LineStatus) -> &'static str {
    match status {
        alp_voice::LineStatus::Pending => "pending",
        alp_voice::LineStatus::Rendering => "rendering",
        alp_voice::LineStatus::Ready => "ready",
        alp_voice::LineStatus::Failed => "failed",
    }
}

impl App {
    /// Story list, line editor, render controls and the tools overlay.
    pub(super) fn stories_workspace(&self, colors: ThemeColors) -> Element<'_, Message> {
        let list = self.story_list(colors);
        let detail = self.story_detail(colors);

        let content =
            column![
                header(
                    "Stories",
                    "Narrate a script, then shape it on the track editor",
                    vec![
                        components::button(
                            "New Story",
                            ButtonKind::Primary,
                            colors,
                            Some(Message::NewStory),
                        ),
                        components::button(
                            "Save",
                            ButtonKind::Ghost,
                            colors,
                            Some(Message::SaveStory),
                        ),
                    ],
                    colors,
                ),
                row![
                    column![list]
                        .width(Length::FillPortion(5))
                        .height(Length::Fill),
                    column![detail]
                        .width(Length::FillPortion(12))
                        .height(Length::Fill),
                ]
                .spacing(f32::from(Spacing::XL))
                .width(Length::Fill)
                .height(Length::Fill),
            ]
            .spacing(f32::from(Spacing::LG))
            .height(Length::Fill);

        if self.stories_tools_open {
            stack![content, self.stories_tools_overlay(colors)]
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            content.into()
        }
    }

    fn story_list(&self, colors: ThemeColors) -> Element<'_, Message> {
        let mut items = column![].spacing(f32::from(Spacing::SM));
        if self.stories.is_empty() {
            items = items.push(components::muted(
                "No stories yet. Create one to start.",
                colors,
            ));
        }
        for (index, story) in self.stories.iter().enumerate() {
            let selected = index == self.selected_story;
            let ready = story
                .lines
                .iter()
                .filter(|line| line.status == alp_voice::LineStatus::Ready)
                .count();
            let meta = format!("{} item(s) · {} ready", story.lines.len(), ready);
            let card = container(
                column![
                    text(story.title.clone())
                        .size(f32::from(TypeScale::HEADING))
                        .font(fonts::semibold())
                        .color(colors.text),
                    components::muted(meta, colors),
                ]
                .spacing(f32::from(Spacing::XXS)),
            )
            .width(Length::Fill)
            .padding(Spacing::MD)
            .style(move |_theme| container::Style {
                background: Some(Background::Color(if selected {
                    colors.surface_raised
                } else {
                    colors.glass_strong
                })),
                border: Border {
                    radius: f32::from(Radius::MD).into(),
                    width: 1.0,
                    color: if selected {
                        colors.border_strong
                    } else {
                        colors.border
                    },
                },
                shadow: iced::Shadow::default(),
                text_color: Some(colors.text),
                snap: true,
            });
            items = items.push(mouse_area(card).on_press(Message::SelectStory(index)));
        }

        let composer = container(
            column![
                components::text_field(
                    "Generate speech for this story…",
                    &self.story_text,
                    colors,
                    Message::StoryLineInput,
                ),
                row![
                    container(components::dropdown(
                        self.profiles
                            .iter()
                            .map(|profile| profile.name.clone())
                            .collect(),
                        self.profile().map(|profile| profile.name.clone()),
                        colors,
                        Message::SelectVoice,
                    ))
                    .width(Length::FillPortion(1)),
                    components::button(
                        "✦",
                        ButtonKind::Primary,
                        colors,
                        Some(Message::AddStoryLine),
                    ),
                ]
                .spacing(f32::from(Spacing::XS))
                .align_y(Alignment::Center),
            ]
            .spacing(f32::from(Spacing::SM)),
        )
        .width(Length::Fill)
        .padding(Spacing::MD)
        .style(move |_theme| colors.card_style());

        column![self.scroll(items, colors).height(Length::Fill), composer,]
            .spacing(f32::from(Spacing::MD))
            .height(Length::Fill)
            .into()
    }

    fn story_detail(&self, colors: ThemeColors) -> Element<'_, Message> {
        let story = self.stories.get(self.selected_story);
        let Some(story) = story else {
            return container(components::muted("Select a story on the left.", colors))
                .padding(Spacing::XL)
                .into();
        };

        let title = story.title.clone();
        let line_count = story.lines.len();
        let ready = story
            .lines
            .iter()
            .filter(|line| line.status == alp_voice::LineStatus::Ready)
            .count();

        let head = row![
            column![
                text(title)
                    .size(f32::from(TypeScale::TITLE))
                    .font(fonts::bold())
                    .color(colors.text),
                components::muted(
                    format!("{line_count} line(s) · {ready} ready · rendered locally"),
                    colors
                ),
            ]
            .spacing(f32::from(Spacing::XXS)),
            Space::new().width(Length::Fill),
            components::button(
                "+  Add",
                ButtonKind::Secondary,
                colors,
                Some(Message::AddStoryLine),
            ),
            components::button(
                if self.generating {
                    "Rendering…"
                } else {
                    "↓  Render Audio"
                },
                ButtonKind::Primary,
                colors,
                Some(Message::RenderStory),
            ),
        ]
        .spacing(f32::from(Spacing::SM))
        .align_y(Alignment::Center);

        let mut lines = column![].spacing(f32::from(Spacing::MD));
        if story.lines.is_empty() {
            lines = lines.push(
                container(
                    column![
                        text("This story is empty")
                            .size(f32::from(TypeScale::HEADING))
                            .font(fonts::semibold())
                            .color(colors.text),
                        components::muted(
                            "Write a line below the list and press the gold button to add it.",
                            colors,
                        ),
                    ]
                    .spacing(f32::from(Spacing::XS)),
                )
                .padding(Spacing::XL)
                .style(move |_theme| colors.well_style()),
            );
        }
        for (index, line) in story.lines.iter().enumerate() {
            let selected = self.selected_story_line == Some(index);
            let voice = self.profile_name(&line.voice_id);
            let line_card = container(
                column![
                    row![
                        components::avatar(&voice, 36.0, colors),
                        text(voice)
                            .size(f32::from(TypeScale::BODY_STRONG))
                            .font(fonts::semibold())
                            .color(colors.text),
                        components::chip("en", colors),
                        Space::new().width(Length::Fill),
                        components::muted(
                            format!("{:.1}x · {}", line.speed, status_label(line.status)),
                            colors
                        ),
                        components::with_tooltip(
                            components::glyph_button(
                                "⋯",
                                16.0,
                                ButtonKind::Ghost,
                                colors,
                                Some(Message::SelectStoryLine(index)),
                            ),
                            "Select this line".to_string(),
                            colors,
                        ),
                        components::with_tooltip(
                            components::glyph_button(
                                "⌫",
                                12.0,
                                ButtonKind::Ghost,
                                colors,
                                Some(Message::DeleteStoryLine(index)),
                            ),
                            "Delete this line".to_string(),
                            colors,
                        ),
                    ]
                    .spacing(f32::from(Spacing::SM))
                    .align_y(Alignment::Center),
                    container(
                        text(line.text.clone())
                            .size(f32::from(TypeScale::BODY))
                            .font(fonts::regular())
                            .color(colors.text),
                    )
                    .width(Length::Fill)
                    .padding(Spacing::MD)
                    .style(move |_theme| colors.well_style()),
                ]
                .spacing(f32::from(Spacing::SM)),
            )
            .width(Length::Fill)
            .padding(Spacing::MD)
            .style(move |_theme| container::Style {
                background: Some(Background::Color(colors.glass_strong)),
                border: Border {
                    radius: f32::from(Radius::MD).into(),
                    width: 1.0,
                    color: if selected {
                        colors.accent
                    } else {
                        colors.border
                    },
                },
                shadow: iced::Shadow::default(),
                text_color: Some(colors.text),
                snap: true,
            });
            lines = lines.push(mouse_area(line_card).on_press(Message::SelectStoryLine(index)));
        }

        column![head, self.scroll(lines, colors).height(Length::Fill)]
            .spacing(f32::from(Spacing::LG))
            .height(Length::Fill)
            .into()
    }

    fn stories_tools_overlay(&self, colors: ThemeColors) -> Element<'_, Message> {
        let panel = container(
            column![
                row![
                    components::card_title("Mixer & Inspector", colors),
                    Space::new().width(Length::Fill),
                    components::glyph_button(
                        "✕",
                        13.0,
                        ButtonKind::Ghost,
                        colors,
                        Some(Message::ToggleStoriesTools),
                    ),
                ]
                .align_y(Alignment::Center),
                card("Timeline", "▤", self.timeline_view(colors), colors),
                card("Mixer", "∑", self.mixer_view(colors), colors),
                card("Inspector", "▥", self.inspector_view(colors), colors),
            ]
            .spacing(f32::from(Spacing::MD)),
        )
        .width(Length::Fixed(460.0))
        .height(Length::Fill)
        .padding(Spacing::LG)
        .style(move |_theme| colors.frame_style());

        container(panel)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding([Spacing::XL, Spacing::XL])
            .align_x(Alignment::End)
            .style(move |_theme| container::Style {
                background: Some(Background::Color(colors.scrim)),
                ..container::Style::default()
            })
            .into()
    }
}
