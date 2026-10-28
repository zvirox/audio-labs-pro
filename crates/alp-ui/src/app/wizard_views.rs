//! First-run configuration wizard and keyboard shortcuts help overlays.

use alp_ui_kit::components;
use alp_ui_kit::overlays;
use alp_ui_kit::tokens::{Radius, Spacing, TypeScale};
use alp_ui_kit::{ButtonKind, ThemeColors, fonts};
use iced::widget::{Space, column, container, row, text};
use iced::{Alignment, Background, Border, Element, Length};

use crate::app::{App, Message};
use crate::glass::GlassMode;

impl App {
    /// Render the first-run onboarding and hardware configuration wizard overlay.
    pub(super) fn first_run_wizard_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let devices_summary = if self.input_devices.is_empty() {
            "Default system audio (PipeWire / ALSA)".to_string()
        } else {
            let shown: Vec<&str> = self
                .input_devices
                .iter()
                .take(3)
                .map(String::as_str)
                .collect();
            let extra = self.input_devices.len().saturating_sub(shown.len());
            if extra > 0 {
                format!("{} · +{extra} more", shown.join(", "))
            } else {
                shown.join(", ")
            }
        };

        let glass_status = match self.glass.mode {
            GlassMode::CompositorBlur => "Active (hardware accelerated)",
            GlassMode::TintFallback => "Fallback (clean tinted glass)",
        };

        let theme_label = match self.theme_mode {
            alp_ui_kit::ThemeMode::Dark => "Dark",
            alp_ui_kit::ThemeMode::Light => "Light",
            alp_ui_kit::ThemeMode::System => "System",
        };

        let step =
            |number: &'static str, title: &'static str, detail: Element<'static, Message>| {
                let header = row![
                    components::chip(number, colors),
                    text(title)
                        .size(f32::from(TypeScale::BODY_STRONG))
                        .font(fonts::semibold())
                        .color(colors.text),
                ]
                .spacing(f32::from(Spacing::SM))
                .align_y(Alignment::Center);
                container(
                    column![header, detail]
                        .spacing(f32::from(Spacing::XS))
                        .padding(Spacing::MD),
                )
                .width(Length::Fill)
                .style(move |_theme| colors.well_style())
            };

        let body = column![
            components::muted(
                "Configure devices, appearance and the local model tier. Nothing leaves this machine.",
                colors,
            ),
            step(
                "01",
                "Audio hardware",
                components::muted(format!("Detected inputs: {devices_summary}"), colors)
            ),
            step(
                "02",
                "System capabilities",
                components::muted(self.hardware_summary.clone(), colors)
            ),
            step(
                "03",
                "Appearance & glass",
                column![
                    components::muted(
                        format!(
                            "Theme {theme_label} · glass {glass_status} · reduce motion {}",
                            if self.reduce_motion {
                                "enabled"
                            } else {
                                "disabled"
                            }
                        ),
                        colors,
                    ),
                    row![
                        components::button(
                            "Toggle theme",
                            ButtonKind::Secondary,
                            Some(Message::ToggleTheme)),
                        components::button(
                            "Toggle motion",
                            ButtonKind::Ghost,
                            Some(Message::ToggleReduceMotion)),
                    ]
                    .spacing(f32::from(Spacing::SM)),
                ]
                .spacing(f32::from(Spacing::XS))
                .into(),
            ),
        ]
        .spacing(f32::from(Spacing::MD));

        overlays::dialog(
            "Welcome to Audio Labs Pro",
            "A local voice studio: record, clone, generate and master without sending audio anywhere.",
            body,
            row![
                Space::new().width(Length::Fill),
                components::button(
                    "Start working",
                    ButtonKind::Primary,
                    Some(Message::FinishFirstRunWizard)
                ),
            ]
            .align_y(Alignment::Center),
            640.0,
            colors,
            Message::FinishFirstRunWizard,
        )
    }

    /// Render the human consent attestation dialog for cloning.
    pub(super) fn consent_modal_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let reference = self
            .record_path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("no reference recording");
        let body = column![
            container(
                row![
                    text("⚖")
                        .size(f32::from(TypeScale::TITLE))
                        .color(colors.accent),
                    components::muted(
                        "A cloned voice profile is only written when a human attests that the speaker gave permission. This record is stored inside the profile bundle.",
                        colors,
                    ),
                ]
                .spacing(f32::from(Spacing::MD))
                .align_y(Alignment::Center)
                .padding(Spacing::MD),
            )
            .width(Length::Fill)
            .style(move |_theme| container::Style {
                background: Some(Background::Color(colors.surface_raised)),
                border: Border {
                    radius: f32::from(Radius::SM).into(),
                    width: 1.0,
                    color: colors.border,
                },
                shadow: iced::Shadow::default(),
                text_color: Some(colors.text),
                snap: true,
            }),
            row![
                column![
                    components::field_label("Speaker", colors),
                    components::text_field(
                        "Speaker name",
                        &self.consent_speaker,
                        colors,
                        Message::ConsentSpeakerInput,
                    ),
                ]
                .spacing(f32::from(Spacing::XS))
                .width(Length::FillPortion(1)),
                column![
                    components::field_label("Attesting engineer", colors),
                    components::text_field(
                        "Your name",
                        &self.consent_attester,
                        colors,
                        Message::ConsentAttesterInput,
                    ),
                ]
                .spacing(f32::from(Spacing::XS))
                .width(Length::FillPortion(1)),
            ]
            .spacing(f32::from(Spacing::MD)),
            components::kicker("Reference", colors),
            components::muted(reference, colors),
            components::muted(
                self.reference_audit_status
                    .clone()
                    .unwrap_or_else(|| "Audit the reference before confirming".to_owned()),
                colors
            ),
        ]
        .spacing(f32::from(Spacing::MD));

        overlays::dialog(
            "Consent to clone",
            "Ask the speaker before you create this profile.",
            body,
            row![
                Space::new().width(Length::Fill),
                components::button(
                    "Cancel",
                    ButtonKind::Secondary,
                    Some(Message::CloseConsentDialog)
                ),
                components::button(
                    "Confirm consent",
                    ButtonKind::Primary,
                    Some(Message::ConfirmConsent)
                ),
            ]
            .spacing(f32::from(Spacing::SM))
            .align_y(Alignment::Center),
            640.0,
            colors,
            Message::CloseConsentDialog,
        )
    }

    /// Render the keyboard shortcuts cheat sheet modal overlay.
    pub(super) fn shortcuts_help_modal_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let shortcuts = [
            (
                "Transport & playback",
                &[
                    ("Space", "Play / pause playback"),
                    ("Home", "Rewind playhead to timeline start"),
                ][..],
            ),
            (
                "Command & panels",
                &[
                    ("Ctrl+K", "Open command palette"),
                    ("F1", "Open keyboard shortcuts"),
                    ("Ctrl+S", "Save project immediately"),
                    ("Ctrl+O", "Import audio media file"),
                    ("Ctrl+Shift+R", "Reset panels to default layout"),
                    ("Ctrl+D", "Toggle dark / light / system theme"),
                    ("Ctrl+Shift+M", "Toggle reduce motion"),
                ][..],
            ),
            (
                "Timeline & editing",
                &[
                    ("Ctrl+Z", "Undo last edit command"),
                    ("Ctrl+Y / Ctrl+Shift+Z", "Redo last undone command"),
                    ("Delete / Backspace", "Delete selected clip or range"),
                    ("Ctrl+B", "Split clip at the playhead"),
                    ("Ctrl+C", "Copy selected range"),
                    ("Ctrl+X", "Cut selected range"),
                    ("Ctrl+V", "Paste clipboard range"),
                    ("R", "Toggle ripple edit mode"),
                ][..],
            ),
            (
                "Voice lab & dictation",
                &[
                    ("Ctrl+Space", "Toggle live microphone dictation"),
                    ("Ctrl+Shift+T", "Transcribe active track clips"),
                ][..],
            ),
        ];

        let mut list = column![].spacing(f32::from(Spacing::MD));

        for (category, items) in shortcuts {
            let mut group =
                column![components::kicker(category, colors)].spacing(f32::from(Spacing::XXS));
            for (key, desc) in items {
                group = group.push(
                    row![
                        text(*key)
                            .size(f32::from(TypeScale::LABEL))
                            .font(fonts::mono())
                            .color(colors.text)
                            .width(Length::Fixed(180.0)),
                        components::muted(*desc, colors),
                    ]
                    .align_y(Alignment::Center)
                    .spacing(f32::from(Spacing::SM))
                    .padding([2.0, 0.0]),
                );
            }
            list = list.push(group);
        }

        let body = column![
            components::muted(
                "Productivity and accessibility shortcuts on this machine.",
                colors,
            ),
            self.scroll(list, colors).height(Length::Fixed(340.0)),
        ]
        .spacing(f32::from(Spacing::SM));

        overlays::dialog(
            "Keyboard shortcuts",
            "Every action is also reachable from the command palette.",
            body,
            row![
                Space::new().width(Length::Fill),
                components::button(
                    "Close",
                    ButtonKind::Secondary,
                    Some(Message::CloseShortcutsHelp)
                ),
            ]
            .align_y(Alignment::Center),
            680.0,
            colors,
            Message::CloseShortcutsHelp,
        )
    }
}
