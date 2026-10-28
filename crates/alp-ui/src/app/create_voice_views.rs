//! Create Voice dialog: sample source, reference transcript and profile details.
//!
//! The dialog is a pure view over [`App`] state. Choosing a source tab changes
//! only presentation; the record path still requires a sample and a consent
//! record before a cloned profile is stored.

use alp_ui_kit::components;
use alp_ui_kit::overlays;
use alp_ui_kit::tokens::{Radius, Spacing, TypeScale};
use alp_ui_kit::{ButtonKind, ThemeColors, fonts};
use iced::widget::{Space, column, container, row, text};
use iced::{Alignment, Background, Border, Element, Length};

use super::{App, CreateVoiceTab, Message, format_time};

const LANGUAGES: [&str; 5] = ["English", "German", "French", "Spanish", "Japanese"];

impl App {
    /// The Create Voice dialog, shown over every workspace.
    pub(super) fn create_voice_dialog_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let tabs = ["Upload", "Record", "System Audio"];
        let source = components::segmented(&tabs, self.create_voice_tab.index(), colors, |index| {
            Message::CreateVoiceTab(match index {
                0 => CreateVoiceTab::Upload,
                2 => CreateVoiceTab::SystemAudio,
                _ => CreateVoiceTab::Record,
            })
        });

        let elapsed = self
            .record_started
            .map_or(0.0, |start| start.elapsed().as_secs_f32());
        let capture = match self.create_voice_tab {
            CreateVoiceTab::Record => self.capture_panel(elapsed, colors),
            CreateVoiceTab::Upload => self.hint_panel(
                "Drop a WAV on the window",
                "Imported takes land in the project. Record on the Record tab to attach a reference to a new profile.",
                colors,
            ),
            CreateVoiceTab::SystemAudio => self.hint_panel(
                "System audio capture is not wired",
                "This build records from the selected input device. Use the Record tab.",
                colors,
            ),
        };

        let left = column![
            source,
            capture,
            components::field_label("Reference Text", colors),
            components::text_field(
                "Enter the exact text spoken in the audio…",
                &self.create_voice_reference,
                colors,
                Message::CreateVoiceReferenceInput,
            ),
        ]
        .spacing(f32::from(Spacing::SM))
        .width(Length::FillPortion(3));

        let avatar = column![
            components::avatar(self.profile_name.as_str(), 72.0, colors),
            components::field_label("Name", colors),
            components::text_field(
                "Linus Sebastian",
                &self.profile_name,
                colors,
                Message::ProfileNameInput
            ),
            components::field_label("Description (optional)", colors),
            components::text_field(
                "The tech tips himself",
                &self.profile_tags,
                colors,
                Message::ProfileTagsInput,
            ),
            components::field_label("Language", colors),
            components::dropdown(
                LANGUAGES
                    .iter()
                    .map(|language| (*language).to_owned())
                    .collect(),
                Some(self.create_voice_language.clone()),
                colors,
                Message::CreateVoiceLanguage,
            ),
        ]
        .spacing(f32::from(Spacing::SM))
        .width(Length::FillPortion(2))
        .align_x(Alignment::Center);

        let body = row![left, right_rule(colors), avatar]
            .spacing(f32::from(Spacing::XL))
            .height(Length::Shrink);

        let footer = row![
            components::button(
                "Design from description",
                ButtonKind::Ghost,
                colors,
                Some(Message::CreateDesignedVoice),
            ),
            Space::new().width(Length::Fill),
            components::button(
                "Cancel",
                ButtonKind::Secondary,
                colors,
                Some(Message::CloseCreateVoice),
            ),
            components::button(
                "Create Profile",
                ButtonKind::Primary,
                colors,
                Some(Message::CreateVoiceSubmit),
            ),
        ]
        .spacing(f32::from(Spacing::SM))
        .align_y(Alignment::Center);

        overlays::dialog(
            "Create voice",
            "Build a new profile from a recording, a dropped sample or a description.",
            body,
            footer,
            960.0,
            colors,
            Message::CloseCreateVoice,
        )
    }

    fn capture_panel(&self, elapsed: f32, colors: ThemeColors) -> Element<'_, Message> {
        let recording = self.recording.is_some();
        let remaining = (30.0 - elapsed).max(0.0);
        let mut bars = row![].spacing(3.0).height(Length::Fixed(64.0));
        for index in 0..56 {
            let phase = (index as f32 * 0.7 + elapsed * 4.0).sin().abs();
            let height = 8.0 + phase * 48.0;
            bars = bars.push(
                container(Space::new())
                    .width(Length::Fixed(4.0))
                    .height(Length::Fixed(height))
                    .style(move |_theme| container::Style {
                        background: Some(Background::Color(iced::Color {
                            a: 0.55,
                            ..colors.accent
                        })),
                        border: Border {
                            radius: 2.0.into(),
                            ..Border::default()
                        },
                        ..container::Style::default()
                    }),
            );
        }

        container(
            column![
                container(bars)
                    .width(Length::Fill)
                    .height(Length::Fixed(96.0))
                    .align_x(Alignment::Center)
                    .align_y(iced::alignment::Vertical::Center),
                text(format_time(elapsed))
                    .size(f32::from(TypeScale::TITLE))
                    .font(fonts::semibold())
                    .color(colors.text),
                components::button(
                    if recording {
                        "■  Stop Recording"
                    } else {
                        "●  Start Recording"
                    },
                    ButtonKind::Primary,
                    colors,
                    Some(Message::ToggleRecord),
                ),
                components::muted(
                    if recording {
                        format!("{} remaining", format_time(remaining))
                    } else {
                        "Record up to 30 seconds of clean speech".to_owned()
                    },
                    colors,
                ),
            ]
            .spacing(f32::from(Spacing::SM))
            .align_x(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fixed(200.0))
        .padding(Spacing::MD)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(colors.glass_strong)),
            border: Border {
                radius: f32::from(Radius::MD).into(),
                width: 1.0,
                color: colors.accent,
            },
            shadow: iced::Shadow::default(),
            text_color: Some(colors.text),
            snap: true,
        })
        .into()
    }

    fn hint_panel(
        &self,
        title: &'static str,
        hint: &'static str,
        colors: ThemeColors,
    ) -> Element<'_, Message> {
        container(
            column![
                text(title)
                    .size(f32::from(TypeScale::HEADING))
                    .font(fonts::semibold())
                    .color(colors.text),
                components::muted(hint, colors),
            ]
            .spacing(f32::from(Spacing::XS))
            .align_x(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fixed(200.0))
        .align_x(Alignment::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(move |_theme| colors.well_style())
        .into()
    }
}

fn right_rule(colors: ThemeColors) -> Element<'static, Message> {
    container(Space::new())
        .width(Length::Fixed(1.0))
        .height(Length::Fill)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(colors.border)),
            ..container::Style::default()
        })
        .into()
}
