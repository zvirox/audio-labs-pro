//! Stories workspace: multi-line scripts, line status and the timeline.

use alp_ui_kit::components;
use alp_ui_kit::tokens::{Spacing, TypeScale};
use alp_ui_kit::{ButtonKind, ThemeColors, fonts};
use iced::widget::{Space, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length};

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
    /// Story list, line editor, render controls and the shared timeline.
    pub(super) fn stories_workspace(&self, colors: ThemeColors) -> Element<'_, Message> {
        let mut story_rows = column![].spacing(f32::from(Spacing::XXS));
        for (index, story) in self.stories.iter().enumerate() {
            let selected = index == self.selected_story;
            story_rows = story_rows.push(
                row![
                    components::button(
                        format!("{}  ·  {}", story.title, story.lines.len()),
                        if selected {
                            ButtonKind::Secondary
                        } else {
                            ButtonKind::Ghost
                        },
                        colors,
                        Some(Message::SelectStory(index))
                    ),
                    Space::new().width(Length::Fill),
                    if selected {
                        components::muted("viewing", colors)
                    } else {
                        components::muted("", colors)
                    },
                ]
                .spacing(f32::from(Spacing::XS))
                .align_y(Alignment::Center),
            );
        }

        let story_list = card(
            "Story Projects",
            "📖",
            column![
                row![
                    self.action_btn("New", ButtonKind::Secondary, Message::NewStory),
                    self.action_btn("Save", ButtonKind::Ghost, Message::SaveStory),
                ]
                .spacing(f32::from(Spacing::XXS)),
                scrollable(story_rows).height(Length::Fill),
            ]
            .spacing(f32::from(Spacing::XS))
            .into(),
            colors,
        );

        let story = self.stories.get(self.selected_story);
        let mut line_rows = column![].spacing(f32::from(Spacing::XS));
        if let Some(story) = story {
            if story.lines.is_empty() {
                line_rows = line_rows.push(components::muted(
                    "This story has no lines yet. Add one below.",
                    colors,
                ));
            }
            for (index, line) in story.lines.iter().enumerate() {
                let selected = self.selected_story_line == Some(index);
                line_rows = line_rows.push(
                    container(
                        column![
                            row![
                                text(format!("{:02}", index + 1))
                                    .size(f32::from(TypeScale::CAPTION))
                                    .font(fonts::mono())
                                    .color(colors.faint),
                                text(self.profile_name(&line.voice_id))
                                    .size(f32::from(TypeScale::CAPTION))
                                    .font(fonts::semibold())
                                    .color(colors.accent_secondary),
                                components::kicker(status_label(line.status), colors),
                                Space::new().width(Length::Fill),
                                components::button(
                                    "Select",
                                    if selected {
                                        ButtonKind::Secondary
                                    } else {
                                        ButtonKind::Ghost
                                    },
                                    colors,
                                    Some(Message::SelectStoryLine(index)),
                                ),
                                components::button(
                                    "✕",
                                    ButtonKind::Ghost,
                                    colors,
                                    Some(Message::DeleteStoryLine(index)),
                                ),
                            ]
                            .spacing(f32::from(Spacing::XXS))
                            .align_y(Alignment::Center),
                            text(line.text.clone())
                                .size(f32::from(TypeScale::BODY))
                                .color(colors.text),
                        ]
                        .spacing(f32::from(Spacing::XXS)),
                    )
                    .padding(Spacing::XS)
                    .style(move |_theme| {
                        if selected {
                            colors.panel_style(true)
                        } else {
                            colors.card_style()
                        }
                    }),
                );
            }
        }

        let lines_card = card(
            "Lines",
            "💬",
            scrollable(line_rows).height(Length::Fill).into(),
            colors,
        );

        let selected_text = story
            .and_then(|story| {
                self.selected_story_line
                    .and_then(|index| story.lines.get(index))
            })
            .map(|line| line.text.clone())
            .unwrap_or_default();
        let title = story.map_or("No story".into(), |story| story.title.clone());

        let composer = container(
            column![
                components::kicker(title, colors),
                components::muted(
                    "Each line is rendered with the profile it names. Rendering stitches ready lines into one take with a short pause between them.",
                    colors
                ),
                text_input(
                    if selected_text.is_empty() {
                        "Write the next line of the story…"
                    } else {
                        &selected_text
                    },
                    &self.story_text
                )
                .on_input(Message::StoryLineInput)
                .padding(Spacing::SM),
                row![
                    self.action_btn("Add line", ButtonKind::Secondary, Message::AddStoryLine),
                    self.action_btn(
                        if self.generating {
                            "Rendering…"
                        } else {
                            "✦ Render story"
                        },
                        ButtonKind::Primary,
                        Message::RenderStory
                    ),
                    Space::new().width(Length::Fill),
                    components::muted(self.voice_status.clone(), colors),
                ]
                .spacing(f32::from(Spacing::XS))
                .align_y(Alignment::Center),
            ]
            .spacing(f32::from(Spacing::XS)),
        )
        .padding(Spacing::MD)
        .style(move |_theme| colors.panel_style(true));

        let top = row![
            column![story_list]
                .width(Length::FillPortion(2))
                .height(Length::Fill),
            column![lines_card]
                .width(Length::FillPortion(3))
                .height(Length::Fill),
            column![composer]
                .width(Length::FillPortion(3))
                .height(Length::Fill),
        ]
        .spacing(f32::from(Spacing::SM))
        .width(Length::Fill)
        .height(Length::FillPortion(3));

        let bottom = row![
            container(card(
                "Transport & Timeline",
                "▤",
                column![self.transport_view(colors), self.timeline_view(colors),]
                    .spacing(f32::from(Spacing::XS))
                    .into(),
                colors
            ))
            .width(Length::FillPortion(3))
            .height(Length::Fill),
            container(card("Mixer", "∑", self.mixer_view(colors), colors))
                .width(Length::FillPortion(2))
                .height(Length::Fill),
            container(card("Inspector", "▥", self.inspector_view(colors), colors))
                .width(Length::FillPortion(3))
                .height(Length::Fill),
        ]
        .spacing(f32::from(Spacing::SM))
        .width(Length::Fill)
        .height(Length::Fixed(330.0));

        column![
            header(
                "Stories",
                "Narrate a script, then edit the result on the timeline",
                vec![self.action_btn(
                    "Generate",
                    ButtonKind::Secondary,
                    Message::SelectWorkflowTab(super::WorkflowTab::Generate)
                )],
                colors,
            ),
            top,
            bottom,
        ]
        .spacing(f32::from(Spacing::SM))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
