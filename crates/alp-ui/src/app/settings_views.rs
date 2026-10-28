//! Models and Settings workspaces.

use alp_media::ExportPreset;
use alp_ui_kit::components;
use alp_ui_kit::tokens::{Spacing, TypeScale};
use alp_ui_kit::{ButtonKind, ThemeColors, fonts};
use iced::widget::{Space, column, container, row, text};
use iced::{Alignment, Element, Length};

use super::workspace_layout::{card, card_auto, header, metric_badge};
use super::{App, Message};

fn kind_label(kind: alp_voice::ModelKind) -> &'static str {
    match kind {
        alp_voice::ModelKind::TtsLight => "TTS · light",
        alp_voice::ModelKind::TtsMedium => "TTS · medium",
        alp_voice::ModelKind::Stt => "STT",
        alp_voice::ModelKind::VoiceClone => "Cloning",
    }
}

fn status_label(status: &alp_voice::ModelStatus) -> String {
    match status {
        alp_voice::ModelStatus::NotInstalled => "not installed".into(),
        alp_voice::ModelStatus::Downloading { progress } => {
            format!("downloading {:.0}%", progress * 100.0)
        }
        alp_voice::ModelStatus::Installed { .. } => "installed".into(),
        alp_voice::ModelStatus::Verified { .. } => "verified".into(),
        alp_voice::ModelStatus::Corrupt { reason } => format!("corrupt: {reason}"),
    }
}

/// A stacked label + monospace path that wraps instead of clipping.
fn path_row<'a>(
    label: &'static str,
    value: impl Into<String>,
    colors: ThemeColors,
) -> Element<'a, Message> {
    column![
        components::kicker(label, colors),
        text(value.into())
            .size(f32::from(TypeScale::CAPTION))
            .font(fonts::mono())
            .color(colors.muted),
    ]
    .spacing(f32::from(Spacing::XXS))
    .into()
}

/// A label/value row in monospace, used for short technical facts.
fn detail_row<'a>(
    label: &'static str,
    value: impl Into<String>,
    colors: ThemeColors,
) -> Element<'a, Message> {
    row![
        components::kicker(label, colors),
        Space::new().width(Length::Fill),
        text(value.into())
            .size(f32::from(TypeScale::CAPTION))
            .font(fonts::mono())
            .color(colors.muted),
    ]
    .align_y(Alignment::Center)
    .into()
}

impl App {
    /// Model catalogue with license gates and disk status.
    pub(super) fn models_workspace(&self, colors: ThemeColors) -> Element<'_, Message> {
        let mut rows = column![].spacing(f32::from(Spacing::XS));
        for manifest in self.model_manager.catalogue() {
            let status = self.model_manager.status(&manifest.id);
            let installed = status.is_ready();
            let size = format!(
                "{:.1} MB",
                manifest.total_size_bytes() as f64 / (1024.0 * 1024.0)
            );
            let license = if manifest
                .license
                .name
                .eq_ignore_ascii_case(&manifest.license.spdx)
            {
                manifest.license.name.clone()
            } else {
                format!("{} · {}", manifest.license.name, manifest.license.spdx)
            };
            rows = rows.push(
                container(
                    column![
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
                        components::muted(manifest.description.clone(), colors),
                        components::muted(
                            format!("{} · v{} · {}", manifest.author, manifest.version, license),
                            colors
                        ),
                        row![
                            components::chip(kind_label(manifest.kind), colors),
                            components::chip(size, colors),
                            Space::new().width(Length::Fill),
                            if manifest.is_builtin {
                                components::chip("built in · no download", colors)
                            } else {
                                self.action_btn(
                                    if manifest.license.accepted {
                                        "License accepted"
                                    } else {
                                        "Accept license"
                                    },
                                    if manifest.license.accepted {
                                        ButtonKind::Ghost
                                    } else {
                                        ButtonKind::Secondary
                                    },
                                    Message::AcceptModelLicense(manifest.id.0.clone()),
                                )
                            },
                            if !manifest.is_builtin && installed {
                                self.action_btn(
                                    "Remove",
                                    ButtonKind::Ghost,
                                    Message::RemoveModel(manifest.id.0.clone()),
                                )
                            } else {
                                Space::new().width(Length::Fixed(0.0)).into()
                            },
                        ]
                        .spacing(f32::from(Spacing::XS))
                        .align_y(Alignment::Center),
                    ]
                    .spacing(f32::from(Spacing::XS)),
                )
                .padding(Spacing::MD)
                .style(move |_theme| colors.well_style()),
            );
        }

        let catalogue = card(
            "Model Catalogue",
            "◈",
            column![
                row![
                    metric_badge(
                        "Models",
                        self.model_manager.catalogue().len().to_string(),
                        colors
                    ),
                    metric_badge(
                        "Offline",
                        if self.models_offline {
                            "Enforced"
                        } else {
                            "Allowed"
                        },
                        colors
                    ),
                    Space::new().width(Length::Fill),
                    components::muted(
                        self.model_manager.storage_dir().display().to_string(),
                        colors
                    ),
                ]
                .spacing(f32::from(Spacing::XS))
                .align_y(Alignment::Center),
                self.scroll(rows, colors).height(Length::Fill),
            ]
            .spacing(f32::from(Spacing::SM))
            .into(),
            colors,
        );

        let policy = card_auto(
            "Download Policy",
            "⊙",
            column![
                components::toggle(
                    "Offline only (no outbound traffic)",
                    self.models_offline,
                    colors,
                    Message::ToggleOfflineModels
                ),
                components::muted(
                    "Audio Labs Pro has no downloader yet. Accepting a license records the decision; files can be placed in the model folder by hand and verified with `alp-cli`.",
                    colors
                ),
                components::muted(
                    "MP3 export stays disabled for licensing reasons. OGG remains deferred until the encoder license is vetted.",
                    colors
                ),
            ]
            .spacing(f32::from(Spacing::SM))
            .into(),
            colors,
        );

        let verification = card_auto(
            "Verification & Storage",
            "⌕",
            column![
                path_row(
                    "Folder",
                    self.model_manager.storage_dir().display().to_string(),
                    colors
                ),
                detail_row("Integrity", "SHA-256 per manifest file", colors),
                detail_row("Offline", "enforced in the app", colors),
                detail_row("CLI", "alp-cli model verify <id>", colors),
                components::muted(
                    "Nothing is fetched in the background. Install a model by copying its files into the folder above, then verify the hashes before enabling it.",
                    colors
                ),
            ]
            .spacing(f32::from(Spacing::XS))
            .into(),
            colors,
        );

        column![
            header(
                "Models",
                "Local model catalogue, licenses and storage policy",
                vec![],
                colors,
            ),
            row![
                column![catalogue]
                    .width(Length::FillPortion(3))
                    .height(Length::Fill),
                column![policy, verification, Space::new().height(Length::Fill),]
                    .spacing(f32::from(Spacing::LG))
                    .width(Length::FillPortion(2))
                    .height(Length::Fill),
            ]
            .spacing(f32::from(Spacing::XL))
            .height(Length::Fill),
        ]
        .spacing(f32::from(Spacing::LG))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    /// Appearance, motion, AI control, storage and session settings.
    pub(super) fn settings_workspace(&self, colors: ThemeColors) -> Element<'_, Message> {
        let appearance = card_auto(
            "Appearance",
            "◐",
            column![
                row![
                    components::toggle(
                        "Reduce motion",
                        self.reduce_motion,
                        colors,
                        Message::ToggleReduceMotion
                    ),
                    Space::new().width(Length::Fill),
                    self.action_btn(
                        format!("Theme: {}", self.theme_mode.as_str()),
                        ButtonKind::Secondary,
                        Message::ToggleTheme
                    ),
                ]
                .spacing(f32::from(Spacing::XS))
                .align_y(Alignment::Center),
                components::muted(
                    format!(
                        "Studio graphite palette · {} · 6/10/14 px radii",
                        self.glass.badge()
                    ),
                    colors
                ),
                components::muted(
                    "Reduce motion also stops panel and overlay transitions; audio playback is never gated.",
                    colors
                ),
            ]
            .spacing(f32::from(Spacing::SM))
            .into(),
            colors,
        );

        let ai = card_auto("AI Control", "⚡", self.ai_activity_view(colors), colors);

        let storage = card_auto(
            "Storage",
            "▤",
            column![
                path_row("Data", self.media_root.display().to_string(), colors),
                path_row("Stories", self.story_dir.display().to_string(), colors),
                path_row(
                    "Profiles",
                    self.profile_store.as_ref().map_or_else(
                        || "unavailable".to_owned(),
                        |store| store.root_dir().display().to_string()
                    ),
                    colors
                ),
                path_row(
                    "Models",
                    self.model_manager.storage_dir().display().to_string(),
                    colors
                ),
                path_row("Autosave", self.session.status(), colors),
            ]
            .spacing(f32::from(Spacing::XXS))
            .into(),
            colors,
        );

        let delivery = card_auto(
            "Delivery",
            "⇲",
            column![
                row![
                    self.action_btn(
                        "Podcast -16 LUFS",
                        ButtonKind::Secondary,
                        Message::ExportPreset(ExportPreset::Podcast)
                    ),
                    self.action_btn(
                        "Audiobook -20 LUFS",
                        ButtonKind::Ghost,
                        Message::ExportPreset(ExportPreset::Audiobook)
                    ),
                    self.action_btn(
                        "Voice-over -24 LUFS",
                        ButtonKind::Ghost,
                        Message::ExportPreset(ExportPreset::VoiceOver)
                    ),
                ]
                .spacing(f32::from(Spacing::XXS)),
                components::muted(
                    self.session
                        .last_export()
                        .unwrap_or("No mix exported in this session"),
                    colors
                ),
            ]
            .spacing(f32::from(Spacing::SM))
            .into(),
            colors,
        );

        let export_queue = card_auto("Export Queue", "⇲", self.export_view(colors), colors);

        let session = card_auto(
            "Session",
            "◈",
            column![
                row![
                    self.action_btn("Save layout", ButtonKind::Secondary, Message::SaveLayout),
                    self.action_btn("Reset layout", ButtonKind::Ghost, Message::ResetLayout),
                    self.action_btn("Shortcuts", ButtonKind::Ghost, Message::OpenShortcutsHelp),
                    self.action_btn(
                        "First-run setup",
                        ButtonKind::Ghost,
                        Message::OpenFirstRunWizard
                    ),
                ]
                .spacing(f32::from(Spacing::XXS)),
                components::muted(
                    self.last_save_error.clone().unwrap_or_else(|| {
                        "Workspace saved automatically after layout changes".to_owned()
                    }),
                    colors
                ),
            ]
            .spacing(f32::from(Spacing::SM))
            .into(),
            colors,
        );

        let mut panel_rows = column![].spacing(f32::from(Spacing::XXS));
        for kind in crate::panels::PanelKind::ALL {
            let open = self.panels.panel(kind).open;
            let body = row![
                text(kind.glyph())
                    .size(f32::from(TypeScale::LABEL))
                    .color(colors.accent),
                text(kind.title())
                    .size(f32::from(TypeScale::LABEL))
                    .font(fonts::regular())
                    .color(colors.text),
                Space::new().width(Length::Fill),
                components::chip(if open { "open" } else { "closed" }, colors),
                components::button(
                    if open { "Hide" } else { "Show" },
                    ButtonKind::Ghost,
                    colors,
                    Some(Message::TogglePanel(kind))
                ),
            ]
            .spacing(f32::from(Spacing::SM))
            .align_y(Alignment::Center)
            .padding([Spacing::XS, Spacing::SM]);
            panel_rows = panel_rows.push(
                container(body)
                    .width(Length::Fill)
                    .style(move |_theme| colors.well_style()),
            );
        }
        let panels = card_auto(
            "Workspace Panels",
            "▤",
            column![
                components::muted(
                    "Panels stay available to the palette and AI clients even when a workspace hides them.",
                    colors
                ),
                panel_rows,
            ]
            .spacing(f32::from(Spacing::XS))
            .into(),
            colors,
        );

        let left = self
            .scroll(
                column![appearance, storage, panels]
                    .spacing(f32::from(Spacing::LG))
                    .width(Length::Fill),
                colors,
            )
            .height(Length::Fill);

        let right = self
            .scroll(
                column![ai, delivery, export_queue, session]
                    .spacing(f32::from(Spacing::LG))
                    .width(Length::Fill),
                colors,
            )
            .height(Length::Fill);

        column![
            header(
                "Settings",
                "Appearance, automation, storage and delivery defaults",
                vec![self.action_btn(
                    "Shortcuts",
                    ButtonKind::Secondary,
                    Message::OpenShortcutsHelp
                )],
                colors,
            ),
            row![
                container(left)
                    .width(Length::FillPortion(1))
                    .height(Length::Fill),
                container(right)
                    .width(Length::FillPortion(1))
                    .height(Length::Fill),
            ]
            .spacing(f32::from(Spacing::XL))
            .height(Length::Fill),
        ]
        .spacing(f32::from(Spacing::LG))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
