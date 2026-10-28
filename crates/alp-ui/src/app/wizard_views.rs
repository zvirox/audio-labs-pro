//! First-run configuration wizard and keyboard shortcuts help overlays.

use alp_ui_kit::components;
use alp_ui_kit::overlays;
use alp_ui_kit::tokens::{Spacing, TypeScale};
use alp_ui_kit::{ButtonKind, ThemeColors, fonts};
use iced::widget::{column, row, scrollable, text};
use iced::{Alignment, Element, Length};

use crate::app::{App, Message};
use crate::glass::GlassMode;

impl App {
    /// Render the first-run onboarding and hardware configuration wizard overlay.
    pub(super) fn first_run_wizard_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let devices_summary = if self.input_devices.is_empty() {
            "Default System Audio (PipeWire / ALSA)".to_string()
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
            GlassMode::CompositorBlur => "Active (Hardware Accelerated)",
            GlassMode::TintFallback => "Fallback (Clean Tinted Glass)",
        };

        let theme_label = match self.theme_mode {
            alp_ui_kit::ThemeMode::Dark => "Dark",
            alp_ui_kit::ThemeMode::Light => "Light",
            alp_ui_kit::ThemeMode::System => "System",
        };

        let body = column![
            components::muted(
                "Initial Setup & Hardware Calibration — Configure devices, appearance, and voice model tier.",
                colors,
            ),
            column![
                text("1. Audio Hardware Setup")
                    .size(f32::from(TypeScale::BODY))
                    .font(fonts::medium())
                    .color(colors.text),
                components::muted(format!("Detected Inputs: {devices_summary}"), colors),
            ]
            .spacing(f32::from(Spacing::XXS)),
            column![
                text("2. System Capabilities & Recommended AI Tier")
                    .size(f32::from(TypeScale::BODY))
                    .font(fonts::medium())
                    .color(colors.text),
                components::muted(&self.hardware_summary, colors),
            ]
            .spacing(f32::from(Spacing::XXS)),
            column![
                text("3. Appearance & Compositor Glass")
                    .size(f32::from(TypeScale::BODY))
                    .font(fonts::medium())
                    .color(colors.text),
                components::muted(
                    format!(
                        "Theme: {theme_label} · Glass Effect: {glass_status} · Reduce Motion: {}",
                        if self.reduce_motion { "Enabled" } else { "Disabled" }
                    ),
                    colors,
                ),
                row![
                    self.action_btn("Toggle Theme", ButtonKind::Secondary, Message::ToggleTheme),
                    self.action_btn(
                        "Toggle Motion",
                        ButtonKind::Ghost,
                        Message::ToggleReduceMotion,
                    ),
                ]
                .spacing(f32::from(Spacing::SM)),
            ]
            .spacing(f32::from(Spacing::XXS)),
        ]
        .spacing(f32::from(Spacing::MD));

        overlays::modal(
            "Welcome to Audio Labs Pro",
            body,
            vec![(
                "Finish Setup & Start Working".to_string(),
                Message::FinishFirstRunWizard,
            )],
            colors,
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
            components::muted(
                "A cloned voice profile is only written when a human attests that the speaker gave permission. This record is stored inside the profile.",
                colors
            ),
            text("Speaker")
                .size(f32::from(TypeScale::CAPTION))
                .font(fonts::semibold())
                .color(colors.faint),
            iced::widget::text_input("Speaker name", &self.consent_speaker)
                .on_input(Message::ConsentSpeakerInput)
                .padding(Spacing::SM),
            text("Attesting engineer")
                .size(f32::from(TypeScale::CAPTION))
                .font(fonts::semibold())
                .color(colors.faint),
            iced::widget::text_input("Your name", &self.consent_attester)
                .on_input(Message::ConsentAttesterInput)
                .padding(Spacing::SM),
            components::muted(format!("Reference: {reference}"), colors),
            components::muted(
                self.reference_audit_status
                    .clone()
                    .unwrap_or_else(|| "Audit the reference before confirming".to_owned()),
                colors
            ),
        ]
        .spacing(f32::from(Spacing::XS));

        overlays::modal(
            "Consent to Clone",
            body,
            vec![
                ("Cancel".to_string(), Message::CloseConsentDialog),
                ("Confirm consent".to_string(), Message::ConfirmConsent),
            ],
            colors,
        )
    }

    /// Render the keyboard shortcuts cheat sheet modal overlay.
    pub(super) fn shortcuts_help_modal_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let shortcuts = [
            (
                "Transport & Playback",
                &[
                    ("Space", "Play / pause playback"),
                    ("Home", "Rewind playhead to timeline start"),
                ][..],
            ),
            (
                "Command & Panels",
                &[
                    ("Ctrl+K", "Open Command Palette"),
                    ("F1", "Open Keyboard Shortcuts Help"),
                    ("Ctrl+S", "Save project immediately"),
                    ("Ctrl+O", "Import audio media file"),
                    ("Ctrl+Shift+R", "Reset panels to default layout"),
                    ("Ctrl+D", "Toggle dark / light / system theme"),
                    ("Ctrl+Shift+M", "Toggle Reduce Motion mode"),
                ][..],
            ),
            (
                "Timeline & Editing",
                &[
                    ("Ctrl+Z", "Undo last edit command"),
                    ("Ctrl+Y / Ctrl+Shift+Z", "Redo last undone command"),
                    ("Delete / Backspace", "Delete selected clip or range"),
                    ("Ctrl+B", "Split clip at current playhead position"),
                    ("Ctrl+C", "Copy selected range to clipboard"),
                    ("Ctrl+X", "Cut selected range to clipboard"),
                    ("Ctrl+V", "Paste clipboard range into timeline"),
                    ("R", "Toggle ripple edit mode"),
                ][..],
            ),
            (
                "Voice Lab & Dictation",
                &[
                    ("Ctrl+Space", "Toggle live microphone dictation"),
                    ("Ctrl+Shift+T", "Transcribe active track clips"),
                ][..],
            ),
        ];

        let mut list = column![].spacing(f32::from(Spacing::MD));

        for (category, items) in shortcuts {
            let mut group = column![
                text(category)
                    .size(f32::from(TypeScale::BODY))
                    .font(fonts::medium())
                    .color(colors.text),
            ]
            .spacing(f32::from(Spacing::XXS));

            for (key, desc) in items {
                let shortcut_row = row![
                    text(*key)
                        .size(f32::from(TypeScale::BODY))
                        .color(colors.text)
                        .width(Length::Fixed(180.0)),
                    components::muted(*desc, colors),
                ]
                .align_y(Alignment::Center)
                .spacing(f32::from(Spacing::SM));
                group = group.push(shortcut_row);
            }
            list = list.push(group);
        }

        let body = column![
            components::muted(
                "Audio Labs Pro productivity and accessibility shortcuts",
                colors,
            ),
            scrollable(list).height(Length::Fixed(340.0)),
        ]
        .spacing(f32::from(Spacing::SM));

        overlays::modal(
            "Keyboard Shortcuts Reference",
            body,
            vec![("Close (Esc)".to_string(), Message::CloseShortcutsHelp)],
            colors,
        )
    }
}
