//! Generate workspace: prompt box, profile picker and take history.

use alp_ui_kit::components;
use alp_ui_kit::tokens::{Spacing, TypeScale};
use alp_ui_kit::{ButtonKind, ThemeColors, fonts};
use iced::widget::{Space, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length};

use super::workspace_layout::{card, card_auto, header};
use super::{App, Message, format_time};

impl App {
    /// Generate workspace: render speech and review takes.
    pub(super) fn generate_workspace(&self, colors: ThemeColors) -> Element<'_, Message> {
        let profile_names: Vec<String> = self
            .profiles
            .iter()
            .map(|profile| profile.name.clone())
            .collect();
        let active_name = self
            .profile()
            .map_or_else(|| "No profile".to_owned(), |profile| profile.name.clone());

        let prompt_card = container(
            column![
                row![
                    components::kicker("Active voice", colors),
                    Space::new().width(Length::Fill),
                    components::muted(
                        format!("{} profile(s) in the library", self.profiles.len()),
                        colors
                    ),
                ]
                .align_y(Alignment::Center),
                components::dropdown(
                    profile_names,
                    Some(active_name.clone()),
                    Message::SelectVoice,
                ),
                text_input("Type what the voice should say…", &self.voice_text)
                    .on_input(Message::GenerateTextInput)
                    .padding(Spacing::SM)
                    .size(f32::from(TypeScale::BODY)),
                row![
                    self.action_btn(
                        if self.generating {
                            "Rendering…"
                        } else {
                            "✦ Generate speech"
                        },
                        ButtonKind::Primary,
                        Message::GenerateSpeech,
                    ),
                    self.action_btn("Play last", ButtonKind::Secondary, Message::PlayHistory(0)),
                    Space::new().width(Length::Fill),
                    components::muted(
                        if self.generating {
                            "Working on a worker thread".to_string()
                        } else {
                            format!("{} · built-in acoustic", self.voice_status)
                        },
                        colors,
                    ),
                ]
                .spacing(f32::from(Spacing::XS))
                .align_y(Alignment::Center),
            ]
            .spacing(f32::from(Spacing::SM)),
        )
        .padding(Spacing::MD)
        .style(move |_theme| colors.panel_style(true));

        let mut history_rows = column![].spacing(f32::from(Spacing::XS));
        if self.history.is_empty() {
            history_rows = history_rows.push(
                container(
                    column![
                        text("No takes yet")
                            .size(f32::from(TypeScale::BODY_STRONG))
                            .font(fonts::semibold())
                            .color(colors.text),
                        components::muted(
                            "Write a line and press Generate. Every take is rendered locally and kept on this machine.",
                            colors
                        ),
                    ]
                    .spacing(f32::from(Spacing::XXS)),
                )
                .padding(Spacing::MD)
                .style(move |_theme| colors.card_style()),
            );
        }
        for (index, item) in self.history.iter().enumerate() {
            let snippet = if item.text.chars().count() > 110 {
                let short: String = item.text.chars().take(110).collect();
                format!("{short}…")
            } else {
                item.text.clone()
            };
            history_rows = history_rows.push(
                container(
                    column![
                        row![
                            text(format!("{} · {}", item.voice_name, item.voice_id))
                                .size(f32::from(TypeScale::BODY))
                                .font(fonts::semibold())
                                .color(colors.text),
                            components::kicker(item.kind, colors),
                            Space::new().width(Length::Fill),
                            components::muted(
                                format!("{} · {}", item.created, format_time(item.duration_secs)),
                                colors
                            ),
                        ]
                        .spacing(f32::from(Spacing::XS))
                        .align_y(Alignment::Center),
                        components::muted(snippet, colors),
                        row![
                            self.action_btn("Play", ButtonKind::Ghost, Message::PlayHistory(index)),
                            self.action_btn(
                                "Insert",
                                ButtonKind::Secondary,
                                Message::InsertHistory(index)
                            ),
                            Space::new().width(Length::Fill),
                            self.action_btn(
                                "Delete",
                                ButtonKind::Ghost,
                                Message::DeleteHistory(index)
                            ),
                        ]
                        .spacing(f32::from(Spacing::XXS))
                        .align_y(Alignment::Center),
                    ]
                    .spacing(f32::from(Spacing::XS)),
                )
                .padding(Spacing::SM)
                .style(move |_theme| colors.card_style()),
            );
        }

        let history_card = card(
            "Take History",
            "⏱",
            column![
                row![
                    components::muted(
                        format!(
                            "{} take(s) · stored in the app data folder",
                            self.history.len()
                        ),
                        colors
                    ),
                    Space::new().width(Length::Fill),
                    self.action_btn("Clear", ButtonKind::Ghost, Message::ClearHistory),
                ]
                .align_y(Alignment::Center),
                scrollable(history_rows).height(Length::Fill),
            ]
            .spacing(f32::from(Spacing::SM))
            .into(),
            colors,
        );

        let engine_card = card_auto(
            "Render Engine",
            "◈",
            column![
                row![
                    components::kicker("Backend", colors),
                    Space::new().width(Length::Fill),
                    text("builtin-acoustic · 48 kHz mono · 24-bit WAV")
                        .size(f32::from(TypeScale::CAPTION))
                        .color(colors.muted),
                ],
                row![
                    components::kicker("Profile", colors),
                    Space::new().width(Length::Fill),
                    text(active_name)
                        .size(f32::from(TypeScale::CAPTION))
                        .color(colors.text),
                ],
                row![
                    components::kicker("Storage", colors),
                    Space::new().width(Length::Fill),
                    text(self.media_root.display().to_string())
                        .size(f32::from(TypeScale::CAPTION))
                        .font(fonts::mono())
                        .color(colors.faint),
                ],
                components::muted(
                    "The embedded acoustic engine is a formant synthesizer: it does not clone a timbre. Add a neural model in Models to change the render path.",
                    colors
                ),
            ]
            .spacing(f32::from(Spacing::XS))
            .into(),
            colors,
        );

        let starters = card_auto(
            "Prompt Starters",
            "✎",
            column![
                components::muted(
                    "Quick drafts that show the shape of a good voice-over line.",
                    colors
                ),
                row![
                    self.action_btn(
                        "Welcome narration",
                        ButtonKind::Ghost,
                        Message::GenerateTextInput(
                            "Welcome back. Today we are listening to a story recorded entirely on this machine.".into()
                        )
                    ),
                    self.action_btn(
                        "Audiobook chapter",
                        ButtonKind::Ghost,
                        Message::GenerateTextInput(
                            "Chapter one. The harbour was quiet when the last boat came in.".into()
                        )
                    ),
                    self.action_btn(
                        "Podcast intro",
                        ButtonKind::Ghost,
                        Message::GenerateTextInput(
                            "You are listening to Audio Labs Pro, the local studio for voice and sound.".into()
                        )
                    ),
                ]
                .spacing(f32::from(Spacing::XS)),
            ]
            .spacing(f32::from(Spacing::XS))
            .into(),
            colors,
        );

        let left = column![
            header(
                "Generate",
                "Local speech synthesis, one take at a time",
                vec![self.action_btn(
                    "Voices",
                    ButtonKind::Secondary,
                    Message::SelectWorkflowTab(super::WorkflowTab::Voices)
                )],
                colors,
            ),
            prompt_card,
            engine_card,
            starters,
            Space::new().height(Length::Fill),
        ]
        .spacing(f32::from(Spacing::SM))
        .width(Length::FillPortion(3))
        .height(Length::Fill);

        let right = column![history_card]
            .spacing(f32::from(Spacing::SM))
            .width(Length::FillPortion(2))
            .height(Length::Fill);

        row![left, right]
            .spacing(f32::from(Spacing::SM))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
