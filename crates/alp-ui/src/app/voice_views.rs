//! Voices workspace: the profile library, inspector and cloning gate.

use alp_ui_kit::components;
use alp_ui_kit::icons;
use alp_ui_kit::tokens::{Spacing, TypeScale};
use alp_ui_kit::{ButtonKind, ThemeColors, fonts};
use iced::widget::{Space, column, row, text};
use iced::{Alignment, Element, Length};

use super::settings_views::status_label;
use super::workspace_layout::{card_auto, header, metric_badge};
use super::{App, Message};

impl App {
    /// Neural-tier card: what is installed, what it offers, and how to load it.
    ///
    /// One row per neural-capable model, each with its voices, a live status
    /// chip and the single next action (download, load, or nothing when the
    /// daemon already serves it). The engine line names the daemon state so
    /// the user never guesses why a load failed.
    fn neural_tier_card(&self, colors: ThemeColors) -> Element<'_, Message> {
        let mut rows = column![].spacing(f32::from(Spacing::SM));
        let neural_models = self
            .model_manager
            .catalogue()
            .iter()
            .filter(|manifest| {
                !manifest.is_builtin
                    && matches!(
                        manifest.kind,
                        alp_voice::ModelKind::TtsMedium
                            | alp_voice::ModelKind::TtsLight
                            | alp_voice::ModelKind::Stt
                    )
            })
            .cloned()
            .collect::<Vec<_>>();

        for manifest in &neural_models {
            let status = self.model_manager.status(&manifest.id);
            let ready = status.is_ready();
            let model_id = manifest.id.0.clone();
            // The daemon keeps synthesis and transcription slots; each row's
            // activity is judged by its own slot, so a declined load of one
            // model never un-chips an engine that still serves.
            let is_stt = manifest.kind == alp_voice::ModelKind::Stt;
            let slot_active = if is_stt {
                self.engine.active_stt_model.as_deref() == Some(model_id.as_str())
            } else {
                self.engine.active_model.as_deref() == Some(model_id.as_str())
            };
            let download_job = self
                .model_jobs
                .get(&model_id)
                .and_then(|job_id| self.jobs.jobs().iter().find(|job| job.id == *job_id));
            let downloading = download_job.is_some_and(|job| job.status.is_running());

            let voices = if manifest.voice_names.is_empty() {
                "No voice vectors listed.".to_owned()
            } else {
                manifest.voice_names.join(" , ")
            };

            let mut body = column![
                row![
                    text(manifest.name.clone())
                        .size(f32::from(TypeScale::BODY_STRONG))
                        .font(fonts::bold())
                        .color(colors.text),
                    Space::new().width(Length::Fill),
                    components::chip(status_label(&status), colors),
                ]
                .spacing(f32::from(Spacing::XS))
                .align_y(Alignment::Center),
                components::muted(voices, colors),
            ]
            .spacing(f32::from(Spacing::XXS));

            if let Some(job) = download_job
                && downloading
            {
                body = body.push(
                    column![
                        components::progress(job.progress, colors),
                        components::muted(
                            format!("{}% - {}", (job.progress * 100.0) as usize, job.detail),
                            colors
                        ),
                    ]
                    .spacing(f32::from(Spacing::XXS)),
                );
            } else if let Some(error) = self.model_errors.get(&model_id) {
                body = body.push(
                    text(error.clone())
                        .size(f32::from(TypeScale::CAPTION))
                        .color(colors.danger),
                );
            }

            let mut actions = row![]
                .spacing(f32::from(Spacing::XS))
                .align_y(Alignment::Center);
            if !downloading {
                if !ready {
                    if manifest.license.accepted {
                        actions = actions.push(components::button(
                            "Download",
                            ButtonKind::Secondary,
                            Some(Message::DownloadModel(model_id.clone())),
                        ));
                    } else {
                        actions = actions.push(components::muted(
                            "Accept the license in the Models panel to enable the download",
                            colors,
                        ));
                    }
                } else if !slot_active {
                    actions = actions.push(components::button(
                        "Load engine",
                        ButtonKind::Primary,
                        Some(Message::LoadNeuralModel(model_id.clone())),
                    ));
                } else {
                    actions = actions.push(components::chip("active engine", colors));
                }
            }
            actions = actions.push(Space::new().width(Length::Fill));
            body = body.push(actions);
            rows = rows.push(body);
        }

        card_auto(
            "Neural Tier",
            icons::MODELS,
            column![
                row![
                    components::kicker("alp-infer daemon", colors),
                    Space::new().width(Length::Fill),
                    components::muted(self.engine.label(), colors),
                    components::button(
                        if self.engine_checking {
                            "Checking..."
                        } else {
                            "Check engine"
                        },
                        ButtonKind::Ghost,
                        if self.engine_checking {
                            None
                        } else {
                            Some(Message::CheckEngine)
                        },
                    ),
                ]
                .spacing(f32::from(Spacing::XS))
                .align_y(Alignment::Center),
                self.scroll(rows, colors).height(Length::Shrink),
                components::muted(
                    "Generate routes through the daemon when it runs; the built-in engine covers offline work. Downloads resume from the byte boundary if interrupted.",
                    colors
                ),
            ]
            .spacing(f32::from(Spacing::XS))
            .into(),
            colors,
        )
    }

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
        let neural = self.neural_tier_card(colors);
        let gate = card_auto(
            "Cloning Gate",
            icons::HEART,
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
            icons::STORAGE,
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
                column![inspector, neural, gate, storage]
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
                    components::button_icon(
                        "Import Voice",
                        icons::ARROW_UP,
                        ButtonKind::Secondary,
                        Some(Message::ImportVoice)
                    ),
                    components::button_icon(
                        "Create Voice",
                        icons::GENERATE,
                        ButtonKind::Primary,
                        Some(Message::OpenCreateVoice)
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
                icons::VOICES,
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
            icons::VOICES,
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
                        Some(Message::SelectProfile(id.clone()))
                    ),
                    components::button(
                        "Export",
                        ButtonKind::Secondary,
                        Some(Message::ExportProfile(id.clone()))
                    ),
                    if is_builtin {
                        Space::new().width(Length::Fixed(0.0)).into()
                    } else {
                        components::button(
                            "Delete",
                            ButtonKind::Ghost,
                            Some(Message::DeleteProfile(id)),
                        )
                    },
                ]
                .spacing(f32::from(Spacing::XS)),
                row![
                    components::button(
                        "Clone last take",
                        ButtonKind::Secondary,
                        Some(Message::OpenConsentDialog)
                    ),
                    components::button(
                        "Audit sample",
                        ButtonKind::Ghost,
                        Some(Message::AuditReferenceAudio)
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
