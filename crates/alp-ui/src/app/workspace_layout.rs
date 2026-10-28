//! Studio shell: navigation rail, docked player and workspace dispatch.
//!
//! The navigation model follows the Voicebox studio layout while the contents
//! stay native Audio Labs Pro: every workspace reads the same [`App`] state and
//! sends [`Message`] values, so no view mutates the project directly.

use alp_ui_kit::components;
use alp_ui_kit::icons;
use alp_ui_kit::tokens::{Control, Radius, Rail, Spacing, TypeScale};
use alp_ui_kit::{ButtonKind, ThemeColors, fonts};
use iced::widget::{Scrollable, Space, column, container, mouse_area, row, scrollable, text};
use iced::{Alignment, Background, Border, Element, Length};
use serde::{Deserialize, Serialize};

use super::{App, Message, format_time};

/// Workflow workspace tab selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTab {
    /// Speech generation, takes and the docked player.
    Generate,
    /// Multi-line stories rendered into one narrated take.
    Stories,
    /// Recordings, segmentation and transcript export.
    Captures,
    /// Voice profile library and voice creation.
    Voices,
    /// Cleanup chain and hosted plugins.
    Effects,
    /// Model catalogue, licenses and offline policy.
    Models,
    /// Appearance, motion, AI control and storage.
    Settings,
}

impl WorkflowTab {
    /// All tabs in display order.
    pub const ALL: [Self; 7] = [
        Self::Generate,
        Self::Stories,
        Self::Captures,
        Self::Voices,
        Self::Effects,
        Self::Models,
        Self::Settings,
    ];

    /// Tab label.
    #[must_use]
    pub fn title(self) -> &'static str {
        match self {
            Self::Generate => "Generate",
            Self::Stories => "Stories",
            Self::Captures => "Captures",
            Self::Voices => "Voices",
            Self::Effects => "Effects",
            Self::Models => "Models",
            Self::Settings => "Settings",
        }
    }

    /// Parse a workspace slug, used by `ALP_START_TAB` smoke runs.
    #[must_use]
    pub fn from_slug(slug: &str) -> Option<Self> {
        match slug.trim().to_ascii_lowercase().as_str() {
            "generate" => Some(Self::Generate),
            "stories" => Some(Self::Stories),
            "captures" => Some(Self::Captures),
            "voices" => Some(Self::Voices),
            "effects" => Some(Self::Effects),
            "models" => Some(Self::Models),
            "settings" => Some(Self::Settings),
            _ => None,
        }
    }

    /// Short icon for the navigation rail.
    #[must_use]
    pub fn glyph(self) -> alp_ui_kit::icons::Glyph {
        use alp_ui_kit::icons;
        match self {
            Self::Generate => icons::GENERATE,
            Self::Stories => icons::STORIES,
            Self::Captures => icons::CAPTURES,
            Self::Voices => icons::VOICES,
            Self::Effects => icons::EFFECTS,
            Self::Models => icons::MODELS,
            Self::Settings => icons::SETTINGS,
        }
    }
}

/// A flat studio card with a heading and body.
pub(super) fn card<'a>(
    title: impl Into<String>,
    icon: alp_ui_kit::icons::Glyph,
    body: Element<'a, Message>,
    colors: ThemeColors,
) -> Element<'a, Message> {
    card_sized(title, icon, body, colors, Length::Fill)
}

/// A card that only takes the height its contents need.
pub(super) fn card_auto<'a>(
    title: impl Into<String>,
    icon: alp_ui_kit::icons::Glyph,
    body: Element<'a, Message>,
    colors: ThemeColors,
) -> Element<'a, Message> {
    card_sized(title, icon, body, colors, Length::Shrink)
}

fn card_sized<'a>(
    title: impl Into<String>,
    icon: alp_ui_kit::icons::Glyph,
    body: Element<'a, Message>,
    colors: ThemeColors,
    height: Length,
) -> Element<'a, Message> {
    let header = row![
        alp_ui_kit::icons::tinted(icon, 13.0, Some(colors.accent)),
        text(title.into())
            .size(f32::from(TypeScale::HEADING))
            .font(fonts::semibold())
            .color(colors.text),
    ]
    .spacing(f32::from(Spacing::SM))
    .align_y(Alignment::Center);

    container(
        column![
            header,
            container(body)
                .width(Length::Fill)
                .height(height)
                .clip(true),
        ]
        .spacing(f32::from(Spacing::MD)),
    )
    .width(Length::Fill)
    .height(height)
    .padding(Spacing::LG)
    .style(move |_theme| colors.card_style())
    .into()
}

/// A compact metric badge used by analysis cards.
pub(super) fn metric_badge<'a, Message: 'a>(
    label: &'static str,
    value: impl Into<String>,
    colors: ThemeColors,
) -> Element<'a, Message> {
    container(
        column![
            text(label)
                .size(f32::from(TypeScale::CAPTION))
                .font(fonts::medium())
                .color(colors.faint),
            text(value.into())
                .size(f32::from(TypeScale::BODY_STRONG))
                .font(fonts::semibold())
                .color(colors.text),
        ]
        .spacing(f32::from(Spacing::XXS)),
    )
    .padding([Spacing::SM, Spacing::MD])
    .style(move |_theme| colors.well_style())
    .into()
}

/// Standard workspace header: title, subtitle and trailing actions.
pub(super) fn header<'a>(
    title: &'static str,
    subtitle: impl Into<String>,
    actions: Vec<Element<'a, Message>>,
    colors: ThemeColors,
) -> Element<'a, Message> {
    let mut trailing = row![]
        .spacing(f32::from(Spacing::SM))
        .align_y(Alignment::Center);
    for action in actions {
        trailing = trailing.push(action);
    }
    row![
        column![
            components::page_title(title, colors),
            components::muted(subtitle, colors),
        ]
        .spacing(f32::from(Spacing::XXS)),
        Space::new().width(Length::Fill),
        trailing,
    ]
    .align_y(Alignment::Center)
    .into()
}

impl App {
    /// Fixed left navigation rail.
    pub(super) fn sidebar_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let logo = container(icons::tinted(
            icons::WAVEFORM,
            22.0,
            Some(colors.accent_text),
        ))
        .width(Length::Fixed(f32::from(Rail::LOGO)))
        .height(Length::Fixed(f32::from(Rail::LOGO)))
        .align_x(Alignment::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(colors.accent)),
            border: Border {
                radius: f32::from(Radius::MD).into(),
                width: 0.0,
                color: iced::Color::TRANSPARENT,
            },
            shadow: iced::Shadow::default(),
            text_color: Some(colors.accent_text),
            snap: true,
        });

        let mut nav = column![].spacing(f32::from(Spacing::XS));
        for tab in WorkflowTab::ALL {
            let active = self.workflow_tab == tab;
            nav = nav.push(components::with_tooltip(
                components::rail_button(tab.glyph(), active, Some(Message::SelectWorkflowTab(tab))),
                tab.title().to_string(),
                colors,
            ));
        }

        let ai = components::with_tooltip(
            components::rail_button(icons::AI, self.ai_enabled, Some(Message::ToggleAiControl)),
            if self.ai_enabled {
                "AI control: active".to_string()
            } else {
                "AI control: standby".to_string()
            },
            colors,
        );

        container(
            column![
                logo,
                Space::new().height(Length::Fixed(f32::from(Spacing::LG))),
                nav,
                Space::new().height(Length::Fill),
                ai,
                components::with_tooltip(
                    components::rail_button(
                        icons::KEYBOARD,
                        false,
                        Some(Message::OpenShortcutsHelp)
                    ),
                    "Keyboard shortcuts".to_string(),
                    colors,
                ),
                text("v1.2")
                    .size(f32::from(TypeScale::CAPTION))
                    .color(colors.faint),
            ]
            .spacing(f32::from(Spacing::XS))
            .align_x(Alignment::Center),
        )
        .width(Length::Fixed(f32::from(Rail::WIDTH)))
        .height(Length::Fill)
        .padding([Spacing::LG, Spacing::MD])
        .style(move |_theme| colors.rail_style())
        .into()
    }

    /// A themed scroll area; every scrolling surface goes through here so the
    /// scrollbar matches the studio palette.
    pub(super) fn scroll<'a>(
        &self,
        content: impl Into<Element<'a, Message>>,
        colors: ThemeColors,
    ) -> Scrollable<'a, Message> {
        scrollable(content).style(move |_theme, _status| components::scroll_style(colors))
    }

    /// A voice profile card shared by Generate and Voices.
    pub(super) fn profile_card(
        &self,
        profile: &alp_voice::VoiceProfile,
        colors: ThemeColors,
    ) -> Element<'_, Message> {
        let selected = profile.id == self.selected_profile;
        let kind = match profile.kind {
            alp_voice::VoiceKind::Builtin => "built-in",
            alp_voice::VoiceKind::Cloned => "cloned",
            alp_voice::VoiceKind::Designed => "designed",
        };
        let description = if profile.tags.is_empty() {
            "Local voice profile".to_owned()
        } else {
            profile.tags.join(" ")
        };
        let is_builtin = profile.kind == alp_voice::VoiceKind::Builtin;
        let id = profile.id.clone();

        let body = column![
            row![
                components::avatar(&profile.name, 34.0, colors),
                column![
                    text(profile.name.clone())
                        .size(f32::from(TypeScale::BODY_STRONG))
                        .font(fonts::semibold())
                        .color(colors.text),
                    components::chip(kind, colors),
                ]
                .spacing(f32::from(Spacing::XXS))
                .align_x(Alignment::Start),
            ]
            .spacing(f32::from(Spacing::SM))
            .align_y(Alignment::Center),
            text(description)
                .size(f32::from(TypeScale::CAPTION))
                .font(fonts::regular())
                .color(colors.muted),
            Space::new().height(Length::Fill),
            row![
                Space::new().width(Length::Fill),
                components::with_tooltip(
                    components::glyph_button_compact(
                        icons::PLAY,
                        12.0,
                        ButtonKind::Ghost,
                        Some(Message::SelectProfile(id.clone()))
                    ),
                    "Use this voice".to_string(),
                    colors,
                ),
                components::with_tooltip(
                    components::glyph_button_compact(
                        icons::ARROW_DOWN,
                        12.0,
                        ButtonKind::Ghost,
                        Some(Message::ExportProfile(id.clone()))
                    ),
                    "Export bundle".to_string(),
                    colors,
                ),
                if is_builtin {
                    Space::new().width(Length::Fixed(28.0)).into()
                } else {
                    components::with_tooltip(
                        components::glyph_button_compact(
                            icons::DELETE_KEY,
                            12.0,
                            ButtonKind::Ghost,
                            Some(Message::DeleteProfile(id)),
                        ),
                        "Delete profile".to_string(),
                        colors,
                    )
                },
            ]
            .spacing(f32::from(Spacing::XXS))
            .align_y(Alignment::Center),
        ]
        .spacing(f32::from(Spacing::XS))
        .height(Length::Fill);

        container(body)
            .width(Length::Fill)
            .height(Length::Fixed(128.0))
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
            })
            .into()
    }

    /// A responsive grid of voice cards that keeps equal column widths even on
    /// a partially filled last row.
    pub(super) fn profile_grid(&self, columns: usize, colors: ThemeColors) -> Element<'_, Message> {
        let columns = columns.max(1);
        let mut grid = column![].spacing(f32::from(Spacing::SM));
        for chunk in self.profiles.chunks(columns) {
            let mut line = row![].spacing(f32::from(Spacing::SM));
            for profile in chunk {
                line = line.push(
                    container(self.profile_card(profile, colors)).width(Length::FillPortion(1)),
                );
            }
            let missing = columns - chunk.len();
            if missing > 0 {
                line = line.push(Space::new().width(Length::FillPortion(missing as u16)));
            }
            grid = grid.push(line);
        }
        grid.into()
    }

    /// Bottom docked player, present on every workspace except Stories.
    pub(super) fn docked_player_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let play_icon = if self.playing {
            icons::PAUSE
        } else {
            icons::PLAY
        };
        let progress = (self.position_secs / self.media_duration.max(1.0)).clamp(0.0, 1.0);
        let waveform = container(self.waveform.view())
            .width(Length::Fill)
            .height(Length::Fixed(48.0))
            .clip(true);

        container(
            row![
                components::glyph_button(
                    play_icon,
                    16.0,
                    ButtonKind::Primary,
                    Some(Message::PlayPause)
                ),
                components::icon_button(icons::SKIP_BACK, Some(Message::Stop)),
                column![
                    row![
                        text(self.player_title.clone())
                            .size(f32::from(TypeScale::BODY))
                            .font(fonts::medium())
                            .color(colors.text),
                        Space::new().width(Length::Fill),
                        text(format!(
                            "{} / {}",
                            format_time(self.position_secs),
                            format_time(self.media_duration)
                        ))
                        .size(f32::from(TypeScale::CAPTION))
                        .font(fonts::mono())
                        .color(colors.muted),
                    ]
                    .align_y(Alignment::Center),
                    waveform,
                ]
                .spacing(f32::from(Spacing::XXS))
                .width(Length::Fill)
                .height(Length::Fill),
                column![
                    components::muted(
                        format!("{:.0} fps", 1000.0 / self.frame_ms.max(0.1)),
                        colors
                    ),
                    row![
                        text("VOL")
                            .size(9.0)
                            .font(fonts::semibold())
                            .color(colors.faint),
                        container(components::themed_slider(
                            (self.gain / 4.0).clamp(0.0, 1.0),
                            colors,
                            |value| Message::SetMasterGain(value * 4.0)
                        ))
                        .width(Length::Fixed(88.0)),
                    ]
                    .spacing(f32::from(Spacing::XS))
                    .align_y(Alignment::Center),
                    components::progress(progress, colors),
                ]
                .spacing(f32::from(Spacing::XS))
                .width(Length::Fixed(150.0))
                .align_x(Alignment::End),
            ]
            .spacing(f32::from(Spacing::MD))
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fixed(f32::from(Control::DOCK)))
        .padding([Spacing::MD, Spacing::LG])
        .style(move |_theme| container::Style {
            background: Some(Background::Color(colors.glass)),
            border: Border {
                width: 1.0,
                color: colors.border,
                radius: 0.0.into(),
            },
            shadow: iced::Shadow::default(),
            text_color: Some(colors.text),
            snap: true,
        })
        .into()
    }

    /// Render the active workflow workspace.
    pub(super) fn workspace_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        match self.workflow_tab {
            WorkflowTab::Generate => self.generate_workspace(colors),
            WorkflowTab::Stories => self.stories_workspace(colors),
            WorkflowTab::Captures => self.captures_workspace(colors),
            WorkflowTab::Voices => self.voices_workspace(colors),
            WorkflowTab::Effects => self.effects_workspace(colors),
            WorkflowTab::Models => self.models_workspace(colors),
            WorkflowTab::Settings => self.settings_workspace(colors),
        }
    }

    /// The studio dock: a player, or the track editor inside Stories.
    pub(super) fn dock_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        match self.workflow_tab {
            WorkflowTab::Stories => self.track_editor_view(colors),
            _ => self.docked_player_view(colors),
        }
    }

    /// Window controls drawn inside the title strip.
    pub(super) fn traffic_lights(&self, _colors: ThemeColors) -> Element<'_, Message> {
        let light = |color: iced::Color, message: Message| -> Element<'_, Message> {
            mouse_area(
                container(Space::new())
                    .width(Length::Fixed(12.0))
                    .height(Length::Fixed(12.0))
                    .style(move |_theme| container::Style {
                        background: Some(Background::Color(color)),
                        border: Border {
                            radius: 6.0.into(),
                            ..Border::default()
                        },
                        ..container::Style::default()
                    }),
            )
            .on_press(message)
            .into()
        };
        row![
            light(
                iced::Color::from_rgb(0.925, 0.282, 0.600),
                Message::WindowClose
            ),
            light(
                iced::Color::from_rgb(0.961, 0.651, 0.137),
                Message::WindowMinimize
            ),
            light(
                iced::Color::from_rgb(0.157, 0.784, 0.251),
                Message::WindowMaximize
            ),
        ]
        .spacing(f32::from(Spacing::SM))
        .align_y(Alignment::Center)
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::WorkflowTab;

    #[test]
    fn slugs_round_trip_for_every_tab() {
        for tab in WorkflowTab::ALL {
            let slug = tab.title().to_ascii_lowercase();
            assert_eq!(WorkflowTab::from_slug(&slug), Some(tab));
        }
        assert_eq!(WorkflowTab::from_slug("nope"), None);
    }
}
