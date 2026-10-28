//! Seven-workspace shell: navigation rail, docked player and workspace dispatch.
//!
//! The navigation model follows the Voicebox studio layout while the contents
//! stay native Audio Labs Pro: every workspace reads the same [`App`] state and
//! sends [`Message`] values, so no view mutates the project directly.

use alp_ui_kit::components;
use alp_ui_kit::tokens::{Spacing, TypeScale};
use alp_ui_kit::{ButtonKind, ThemeColors, fonts};
use iced::widget::{Space, column, container, row, text};
use iced::{Alignment, Element, Length};
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

    /// Short icon glyph for the navigation rail.
    #[must_use]
    pub fn glyph(self) -> &'static str {
        match self {
            Self::Generate => "✦",
            Self::Stories => "▤",
            Self::Captures => "◉",
            Self::Voices => "◇",
            Self::Effects => "≋",
            Self::Models => "◈",
            Self::Settings => "⚙",
        }
    }
}

/// A raised card with a heading and optional trailing controls.
pub(super) fn card<'a>(
    title: impl Into<String>,
    glyph: &'static str,
    body: Element<'a, Message>,
    colors: ThemeColors,
) -> Element<'a, Message> {
    card_sized(title, glyph, body, colors, Length::Fill)
}

/// A raised card that only takes the height its contents need.
pub(super) fn card_auto<'a>(
    title: impl Into<String>,
    glyph: &'static str,
    body: Element<'a, Message>,
    colors: ThemeColors,
) -> Element<'a, Message> {
    card_sized(title, glyph, body, colors, Length::Shrink)
}

fn card_sized<'a>(
    title: impl Into<String>,
    glyph: &'static str,
    body: Element<'a, Message>,
    colors: ThemeColors,
    height: Length,
) -> Element<'a, Message> {
    let header = row![
        text(glyph).size(13.0).color(colors.accent),
        text(title.into())
            .size(f32::from(TypeScale::BODY_STRONG))
            .font(fonts::semibold())
            .color(colors.text),
    ]
    .spacing(f32::from(Spacing::XS))
    .align_y(Alignment::Center)
    .padding([Spacing::XS, Spacing::SM]);

    container(
        column![
            header,
            container(body)
                .padding([Spacing::XXS, Spacing::SM])
                .width(Length::Fill)
                .height(height)
                .clip(true),
        ]
        .spacing(0.0),
    )
    .width(Length::Fill)
    .height(height)
    .style(move |_theme| colors.panel_style(false))
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
                .font(fonts::semibold())
                .color(colors.faint),
            text(value.into())
                .size(f32::from(TypeScale::BODY_STRONG))
                .font(fonts::semibold())
                .color(colors.text),
        ]
        .spacing(f32::from(Spacing::XXS)),
    )
    .padding([Spacing::XS, Spacing::SM])
    .style(move |_theme| colors.card_style())
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
        .spacing(f32::from(Spacing::XS))
        .align_y(Alignment::Center);
    for action in actions {
        trailing = trailing.push(action);
    }
    row![
        column![
            text(title)
                .size(f32::from(TypeScale::TITLE))
                .font(fonts::bold())
                .color(colors.text),
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
        let logo = container(
            column![
                text("◈").size(22.0).color(colors.accent),
                text("ALP")
                    .size(9.0)
                    .font(fonts::semibold())
                    .color(colors.faint),
            ]
            .spacing(0.0)
            .align_x(Alignment::Center),
        )
        .padding([Spacing::XS, Spacing::XXS]);

        let mut nav = column![].spacing(f32::from(Spacing::XXS));
        for tab in WorkflowTab::ALL {
            let active = self.workflow_tab == tab;
            nav = nav.push(components::with_tooltip(
                components::glyph_button(
                    tab.glyph(),
                    17.0,
                    if active {
                        ButtonKind::Primary
                    } else {
                        ButtonKind::Ghost
                    },
                    colors,
                    Some(Message::SelectWorkflowTab(tab)),
                ),
                tab.title().to_string(),
                colors,
            ));
        }

        let ai = components::with_tooltip(
            components::glyph_button(
                if self.ai_enabled { "⚡" } else { "○" },
                15.0,
                if self.ai_enabled {
                    ButtonKind::Secondary
                } else {
                    ButtonKind::Ghost
                },
                colors,
                Some(Message::ToggleAiControl),
            ),
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
                Space::new().height(Length::Fixed(f32::from(Spacing::XS))),
                nav,
                Space::new().height(Length::Fill),
                ai,
                components::with_tooltip(
                    components::glyph_button(
                        "?",
                        15.0,
                        ButtonKind::Ghost,
                        colors,
                        Some(Message::OpenShortcutsHelp),
                    ),
                    "Keyboard shortcuts".to_string(),
                    colors,
                ),
                text("v1.2")
                    .size(f32::from(TypeScale::CAPTION))
                    .color(colors.faint),
            ]
            .spacing(f32::from(Spacing::XXS))
            .align_x(Alignment::Center),
        )
        .width(Length::Fixed(68.0))
        .height(Length::Fill)
        .padding([Spacing::SM, Spacing::XXS])
        .style(move |_theme| colors.panel_style(false))
        .into()
    }

    /// Bottom docked player, present on every workspace.
    pub(super) fn docked_player_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let play_glyph = if self.playing { "⏸" } else { "▶" };
        let progress = (self.position_secs / self.media_duration.max(1.0)).clamp(0.0, 1.0);
        let now_playing = if self.voice_status.is_empty() {
            "No take loaded".to_owned()
        } else {
            self.voice_status.clone()
        };

        container(
            row![
                components::button(
                    play_glyph,
                    ButtonKind::Primary,
                    colors,
                    Some(Message::PlayPause),
                ),
                components::icon_button("⏮", colors, Some(Message::Stop)),
                text(format_time(self.position_secs))
                    .size(f32::from(TypeScale::CAPTION))
                    .font(fonts::mono())
                    .color(colors.text),
                container(components::progress(progress, colors))
                    .width(Length::FillPortion(2))
                    .align_y(iced::alignment::Vertical::Center),
                components::muted(format_time(self.media_duration), colors),
                Space::new().width(Length::Fixed(f32::from(Spacing::SM))),
                components::muted(now_playing, colors),
                Space::new().width(Length::Fill),
                components::kicker(
                    format!("{:.0} fps", 1000.0 / self.frame_ms.max(0.1)),
                    colors
                ),
            ]
            .spacing(f32::from(Spacing::SM))
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fixed(54.0))
        .padding([Spacing::XS, Spacing::MD])
        .style(move |_theme| colors.card_style())
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
