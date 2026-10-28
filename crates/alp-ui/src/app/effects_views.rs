//! Effects workspace: cleanup chain, hosted plugins and signal path notes.

use alp_ui_kit::components;
use alp_ui_kit::icons;
use alp_ui_kit::tokens::Spacing;
use alp_ui_kit::{ButtonKind, ThemeColors};
use iced::widget::{Space, column, container, row};
use iced::{Alignment, Element, Length};

use super::workspace_layout::{card, card_auto, header, metric_badge};
use super::{App, Message};

impl App {
    /// Voice cleanup chain, plugin browser and mastering targets.
    pub(super) fn effects_workspace(&self, colors: ThemeColors) -> Element<'_, Message> {
        let enabled = self.effects.iter().filter(|effect| effect.enabled).count();
        let chain = card(
            "Cleanup Chain",
            icons::WAVEFORM,
            self.effects_view(colors),
            colors,
        );

        let path = card_auto(
            "Signal Path",
            icons::SIGMA,
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

        let plugin_count = self.discovered_plugins.len();
        let plugins = card(
            "Hosted Plugins",
            icons::LAYERS,
            column![
                row![
                    components::muted(
                        format!("{plugin_count} discovered CLAP/LV2 plugin(s)"),
                        colors
                    ),
                    Space::new().width(Length::Fill),
                    components::button(
                        "Rescan",
                        ButtonKind::Secondary,
                        Some(Message::RescanPlugins)),
                ]
                .align_y(Alignment::Center),
                components::muted(
                    "Plugins load inside the sandboxed alp-plugin host. Inserting a plugin adds it to the cleanup chain on the active track.",
                    colors
                ),
                container(self.plugin_list_view(colors))
                    .width(Length::Fill)
                    .padding(Spacing::XS)
                    .style(move |_theme| colors.well_style()),
            ]
            .spacing(f32::from(Spacing::SM))
            .into(),
            colors,
        );

        let left = column![chain]
            .width(Length::FillPortion(3))
            .height(Length::Fill);
        let right = column![path, plugins]
            .spacing(f32::from(Spacing::LG))
            .width(Length::FillPortion(2))
            .height(Length::Fill);

        column![
            header(
                "Effects",
                "Vocal cleanup, plugin hosting and mastering targets",
                vec![components::button(
                    "Apply cleanup preset",
                    ButtonKind::Primary,
                    Some(Message::ApplyVoiceCleanupPreset)
                )],
                colors,
            ),
            row![left, right]
                .spacing(f32::from(Spacing::XL))
                .width(Length::Fill)
                .height(Length::Fill),
        ]
        .spacing(f32::from(Spacing::LG))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
