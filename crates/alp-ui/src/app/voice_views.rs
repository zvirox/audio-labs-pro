//! Voices workspace: the profile library, inspector and cloning gate.

use alp_ui_kit::components;
use alp_ui_kit::tokens::{Spacing, TypeScale};
use alp_ui_kit::{ButtonKind, ThemeColors, fonts};
use iced::widget::{Space, column, row, text};
use iced::{Alignment, Element, Length};

use super::workspace_layout::{card_auto, header, metric_badge};
use super::{App, Message};

impl App {
    /// Voice library, inspector and the consent gate for clones.
    pub(super) fn voices_workspace(&self, colors: ThemeColors) -> Element<'_, Message> {
        let grid = self.profile_grid(4, colors);

        let library = column![
            self.scroll(grid, colors).height(Length::Fill),
            components::muted(
                "Profiles live in the XDG data directory and are shared by every project. Drop a .alpvoice bundle on the window to import it.",
                colors,
            ),
        ]
        .spacing(f32::from(Spacing::MD))
        .height(Length::Fill);

        let inspector = self.voice_inspector(colors);
        let gate = card_auto(
            "Cloning Gate",
            "⚖",
            column![
                components::muted(
                    "Audio Labs Pro never ships a cloned profile without a consent record from a human. The record is stored inside the profile bundle.",
                    colors
                ),
                row![
                    components::kicker("Hardware", colors),
                    Space::new().width(Length::Fill),
                    components::muted(self.hardware_summary.clone(), colors),
                ],
                components::muted(
                    "The built-in backend does not clone timbre yet; cloned profiles render with the neutral acoustic voice.",
                    colors
                ),
            ]
            .spacing(f32::from(Spacing::XS))
            .into(),
            colors,
        );
        let storage = card_auto(
            "Profile Storage",
            "▤",
            column![
                components::muted(
                    self.profile_store.as_ref().map_or_else(
                        || "Profile store unavailable".to_owned(),
                        |store| store.root_dir().display().to_string()
                    ),
                    colors
                ),
                components::muted(
                    "One folder per profile. `.alpvoice` bundles hold the JSON, reference audio and consent record.",
                    colors
                ),
            ]
            .spacing(f32::from(Spacing::XS))
            .into(),
            colors,
        );

        let right = self
            .scroll(
                column![inspector, gate, storage]
                    .spacing(f32::from(Spacing::LG))
                    .width(Length::Fill),
                colors,
            )
            .width(Length::FillPortion(2))
            .height(Length::Fill);

        let left = column![library]
            .width(Length::FillPortion(3))
            .height(Length::Fill);

        column![
            header(
                "Voices",
                "A local library of built-in, designed and cloned profiles",
                vec![
                    components::button(
                        "↑  Import Voice",
                        ButtonKind::Secondary,
                        colors,
                        Some(Message::ImportVoice),
                    ),
                    components::button(
                        "✦  Create Voice",
                        ButtonKind::Primary,
                        colors,
                        Some(Message::OpenCreateVoice),
                    ),
                ],
                colors,
            ),
            row![left, right]
                .spacing(f32::from(Spacing::XL))
                .width(Length::Fill)
                .height(Length::Fill),
        ]
        .spacing(f32::from(Spacing::LG))
        .height(Length::Fill)
        .into()
    }

    fn voice_inspector(&self, colors: ThemeColors) -> Element<'_, Message> {
        let Some(profile) = self.profile().cloned() else {
            return card_auto(
                "Voice Profile",
                "◇",
                components::muted("Select a profile to inspect it.", colors),
                colors,
            );
        };
        let kind = match profile.kind {
            alp_voice::VoiceKind::Builtin => "Built-in",
            alp_voice::VoiceKind::Cloned => "Cloned",
            alp_voice::VoiceKind::Designed => "Designed",
        };
        let consent = match (&profile.kind, &profile.consent) {
            (alp_voice::VoiceKind::Cloned, Some(_)) => "Consent on file",
            (alp_voice::VoiceKind::Cloned, None) => "Consent missing",
            _ => "No consent required",
        };
        let tags = if profile.tags.is_empty() {
            "No tags".to_owned()
        } else {
            profile.tags.join(" · ")
        };
        let is_builtin = profile.kind == alp_voice::VoiceKind::Builtin;
        let id = profile.id.clone();

        card_auto(
            "Voice Profile",
            "◇",
            column![
                row![
                    components::avatar(&profile.name, 56.0, colors),
                    column![
                        text(profile.name.clone())
                            .size(f32::from(TypeScale::TITLE))
                            .font(fonts::semibold())
                            .color(colors.text),
                        components::muted(tags, colors),
                    ]
                    .spacing(f32::from(Spacing::XXS)),
                ]
                .spacing(f32::from(Spacing::MD))
                .align_y(Alignment::Center),
                row![
                    metric_badge("KIND", kind, colors),
                    metric_badge(
                        "SPEED",
                        format!("{:.2}x", profile.default_params.speed),
                        colors
                    ),
                    metric_badge(
                        "PITCH",
                        format!("{:.1} st", profile.default_params.pitch),
                        colors
                    ),
                ]
                .spacing(f32::from(Spacing::XS)),
                row![
                    components::kicker("Consent", colors),
                    components::chip(consent, colors),
                    Space::new().width(Length::Fill),
                ]
                .spacing(f32::from(Spacing::XS))
                .align_y(Alignment::Center),
                row![
                    components::button(
                        "Use in Generate",
                        ButtonKind::Primary,
                        colors,
                        Some(Message::SelectProfile(id.clone())),
                    ),
                    components::button(
                        "Export",
                        ButtonKind::Secondary,
                        colors,
                        Some(Message::ExportProfile(id.clone())),
                    ),
                    if is_builtin {
                        Space::new().width(Length::Fixed(0.0)).into()
                    } else {
                        components::button(
                            "Delete",
                            ButtonKind::Ghost,
                            colors,
                            Some(Message::DeleteProfile(id)),
                        )
                    },
                ]
                .spacing(f32::from(Spacing::XS)),
                row![
                    components::button(
                        "Clone last take",
                        ButtonKind::Secondary,
                        colors,
                        Some(Message::OpenConsentDialog),
                    ),
                    components::button(
                        "Audit sample",
                        ButtonKind::Ghost,
                        colors,
                        Some(Message::AuditReferenceAudio),
                    ),
                    Space::new().width(Length::Fill),
                ]
                .spacing(f32::from(Spacing::XS)),
            ]
            .spacing(f32::from(Spacing::MD))
            .into(),
            colors,
        )
    }
}
