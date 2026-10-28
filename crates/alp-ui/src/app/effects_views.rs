//! Effects workspace: cleanup chain, hosted plugins and signal path notes.

use alp_ui_kit::components;
use alp_ui_kit::tokens::Spacing;
use alp_ui_kit::{ButtonKind, ThemeColors};
use iced::widget::{Space, column, row};
use iced::{Alignment, Element, Length};

use super::workspace_layout::{card, card_auto, header, metric_badge};
use super::{App, Message};

impl App {
    /// Voice cleanup chain, plugin browser and mastering targets.
    pub(super) fn effects_workspace(&self, colors: ThemeColors) -> Element<'_, Message> {
        let enabled = self.effects.iter().filter(|effect| effect.enabled).count();
        let chain = card("Cleanup Chain", "🪄", self.effects_view(colors), colors);

        let path = card_auto(
            "Signal Path",
            "∑",
            column![
                row![
                    metric_badge(
                        "Stages",
                        format!("{enabled}/{}", self.effects.len()),
                        colors
                    ),
                    metric_badge("Target", "-16 LUFS", colors),
                    metric_badge("Ceiling", "-1.0 dBFS", colors),
                ]
                .spacing(f32::from(Spacing::XS)),
                components::muted(
                    "Chain order is fixed for the voice-cleanup preset: hum, clicks, EQ, de-ess, compression, limiter. The chain is prepared here and runs through the DSP crate in the CLI and export paths.",
                    colors
                ),
            ]
            .spacing(f32::from(Spacing::SM))
            .into(),
            colors,
        );

        let plugins = card(
            "Hosted Plugins",
            "◇",
            column![
                row![
                    components::muted(
                        format!(
                            "{} discovered CLAP/LV2 plugin(s)",
                            self.discovered_plugins.len()
                        ),
                        colors
                    ),
                    Space::new().width(Length::Fill),
                    self.action_btn("Rescan", ButtonKind::Secondary, Message::RescanPlugins),
                ]
                .align_y(Alignment::Center),
                components::muted(
                    "Plugins load inside the sandboxed alp-plugin host. Inserting a plugin adds it to the cleanup chain on the active track.",
                    colors
                ),
                self.plugin_list_view(colors),
            ]
            .spacing(f32::from(Spacing::SM))
            .into(),
            colors,
        );

        let left = column![
            header(
                "Effects",
                "Vocal cleanup, plugin hosting and mastering targets",
                vec![self.action_btn(
                    "Cleanup preset",
                    ButtonKind::Primary,
                    Message::ApplyVoiceCleanupPreset
                )],
                colors,
            ),
            chain,
            Space::new().height(Length::Fill),
        ]
        .spacing(f32::from(Spacing::SM))
        .width(Length::FillPortion(3))
        .height(Length::Fill);

        let right = column![path, plugins]
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
