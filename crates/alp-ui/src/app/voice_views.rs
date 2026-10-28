//! Voices workspace: the profile library and voice-creation pipeline.

use alp_ui_kit::components;
use alp_ui_kit::tokens::{Spacing, TypeScale};
use alp_ui_kit::{ButtonKind, ThemeColors, fonts};
use iced::widget::{Space, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length};

use super::workspace_layout::{card, card_auto, header};
use super::{App, Message};

impl App {
    /// Voice library, designed/cloned creation and reference quality gates.
    pub(super) fn voices_workspace(&self, colors: ThemeColors) -> Element<'_, Message> {
        let mut cards = column![].spacing(f32::from(Spacing::XS));
        for profile in &self.profiles {
            let selected = profile.id == self.selected_profile;
            let kind = match profile.kind {
                alp_voice::VoiceKind::Builtin => "Built-in",
                alp_voice::VoiceKind::Cloned => "Cloned",
                alp_voice::VoiceKind::Designed => "Designed",
            };
            let tags = if profile.tags.is_empty() {
                "no tags".to_owned()
            } else {
                profile.tags.join(" · ")
            };
            let consent = match (&profile.kind, &profile.consent) {
                (alp_voice::VoiceKind::Cloned, Some(_)) => "Consent on file",
                (alp_voice::VoiceKind::Cloned, None) => "Consent missing",
                _ => "No consent required",
            };
            cards = cards.push(
                container(
                    column![
                        row![
                            text(if selected { "●" } else { "○" })
                                .size(12.0)
                                .color(if selected {
                                    colors.accent
                                } else {
                                    colors.faint
                                }),
                            text(profile.name.clone())
                                .size(f32::from(TypeScale::BODY_STRONG))
                                .font(fonts::bold())
                                .color(colors.text),
                            components::kicker(kind, colors),
                            Space::new().width(Length::Fill),
                            components::muted(
                                format!(
                                    "{:.2}x · {:.1}st",
                                    profile.default_params.speed, profile.default_params.pitch
                                ),
                                colors
                            ),
                        ]
                        .spacing(f32::from(Spacing::XS))
                        .align_y(Alignment::Center),
                        components::muted(tags, colors),
                        row![
                            self.action_btn(
                                if selected { "Selected" } else { "Use" },
                                if selected {
                                    ButtonKind::Primary
                                } else {
                                    ButtonKind::Secondary
                                },
                                Message::SelectProfile(profile.id.clone()),
                            ),
                            self.action_btn(
                                "Export",
                                ButtonKind::Ghost,
                                Message::ExportProfile(profile.id.clone()),
                            ),
                            Space::new().width(Length::Fill),
                            components::muted(consent, colors),
                            if profile.kind == alp_voice::VoiceKind::Builtin {
                                Space::new().width(Length::Fixed(0.0)).into()
                            } else {
                                self.action_btn(
                                    "Delete",
                                    ButtonKind::Ghost,
                                    Message::DeleteProfile(profile.id.clone()),
                                )
                            },
                        ]
                        .spacing(f32::from(Spacing::XXS))
                        .align_y(Alignment::Center),
                    ]
                    .spacing(f32::from(Spacing::XS)),
                )
                .padding(Spacing::SM)
                .style(move |_theme| {
                    if selected {
                        colors.panel_style(true)
                    } else {
                        colors.card_style()
                    }
                }),
            );
        }

        let library = card(
            "Voice Library",
            "👥",
            column![
                components::muted(
                    "Profiles live in the XDG data directory and are shared by every project. Drop a .alpvoice bundle on the window to import it.",
                    colors
                ),
                scrollable(cards).height(Length::Fill),
            ]
            .spacing(f32::from(Spacing::SM))
            .into(),
            colors,
        );

        let reference = self.record_path.as_ref().map_or_else(
            || "No reference take yet".to_owned(),
            |path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("reference.wav")
                    .to_owned()
            },
        );
        let audit = self
            .reference_audit_status
            .clone()
            .unwrap_or_else(|| "Not audited".to_owned());

        let create = card_auto(
            "Create a Voice",
            "✦",
            column![
                components::kicker("Name the voice", colors),
                text_input("Studio Voice", &self.profile_name)
                    .on_input(Message::ProfileNameInput)
                    .padding(Spacing::SM),
                components::kicker("Describe the timbre", colors),
                text_input(
                    "warm, calm, documentary narration",
                    &self.profile_tags
                )
                .on_input(Message::ProfileTagsInput)
                .padding(Spacing::SM),
                components::muted(
                    "Design maps plain-language traits to speed, pitch and energy. Cloning reuses the last recorded take and requires a consent record.",
                    colors
                ),
                row![
                    self.action_btn(
                        "✦ Design voice",
                        ButtonKind::Primary,
                        Message::CreateDesignedProfile
                    ),
                    self.action_btn(
                        "Clone last take",
                        ButtonKind::Secondary,
                        Message::OpenConsentDialog
                    ),
                ]
                .spacing(f32::from(Spacing::XS)),
                Space::new().height(Length::Fixed(f32::from(Spacing::SM))),
                row![
                    components::kicker("Reference", colors),
                    Space::new().width(Length::Fill),
                    text(reference)
                        .size(f32::from(TypeScale::CAPTION))
                        .font(fonts::mono())
                        .color(colors.muted),
                ]
                .align_y(Alignment::Center),
                row![
                    self.action_btn(
                        if self.recording.is_some() {
                            "■ Stop"
                        } else {
                            "🎙 Record sample"
                        },
                        if self.recording.is_some() {
                            ButtonKind::Danger
                        } else {
                            ButtonKind::Secondary
                        },
                        Message::ToggleRecord
                    ),
                    self.action_btn(
                        "Audit quality",
                        ButtonKind::Ghost,
                        Message::AuditReferenceAudio
                    ),
                ]
                .spacing(f32::from(Spacing::XS)),
                components::muted(audit, colors),
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
                    "One folder per profile; `.alpvoice` bundles are ZIP archives with the profile JSON, reference audio and consent record.",
                    colors
                ),
            ]
            .spacing(f32::from(Spacing::XS))
            .into(),
            colors,
        );

        let right = column![
            create,
            card(
                "Cloning Gate",
                "⚖",
                column![
                    components::muted(
                        "Audio Labs Pro never ships a cloned profile without a signed consent record from a human. The record is stored inside the profile bundle.",
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
                colors
            ),
            storage,
            Space::new().height(Length::Fill),
        ]
        .spacing(f32::from(Spacing::SM))
        .width(Length::FillPortion(2))
        .height(Length::Fill);

        let left = column![
            header(
                "Voices",
                "A local library of built-in, designed and cloned profiles",
                vec![self.action_btn(
                    "Export bundle",
                    ButtonKind::Secondary,
                    Message::ExportProfile(self.selected_profile.clone())
                )],
                colors,
            ),
            library,
        ]
        .spacing(f32::from(Spacing::SM))
        .width(Length::FillPortion(3))
        .height(Length::Fill);

        row![left, right]
            .spacing(f32::from(Spacing::SM))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
