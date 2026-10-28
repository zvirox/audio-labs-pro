//! Captures workspace: recordings, input meters and transcript editing.

use alp_ui_kit::components;
use alp_ui_kit::tokens::{Spacing, TypeScale};
use alp_ui_kit::{ButtonKind, ThemeColors, fonts};
use iced::widget::{Space, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

use super::workspace_layout::{card, header};
use super::{App, Message, format_time};

impl App {
    /// Captures workspace: record, audit levels and edit the transcript.
    pub(super) fn captures_workspace(&self, colors: ThemeColors) -> Element<'_, Message> {
        let mut rows = column![].spacing(f32::from(Spacing::XXS));
        if self.captures.is_empty() {
            rows = rows.push(components::muted(
                "No captures yet. Press Record in the capture card.",
                colors,
            ));
        }
        for (index, capture) in self.captures.iter().enumerate() {
            let selected = index == self.selected_capture;
            rows = rows.push(
                row![
                    text(if selected { "●" } else { "○" })
                        .size(11.0)
                        .color(if selected {
                            colors.accent
                        } else {
                            colors.faint
                        }),
                    text(capture.name.clone())
                        .size(f32::from(TypeScale::BODY))
                        .font(fonts::medium())
                        .color(colors.text),
                    Space::new().width(Length::Fill),
                    components::muted(
                        format!(
                            "{} · peak {:.0}%",
                            format_time(capture.duration_secs),
                            capture.peak * 100.0
                        ),
                        colors
                    ),
                    components::button(
                        "Select",
                        if selected {
                            ButtonKind::Secondary
                        } else {
                            ButtonKind::Ghost
                        },
                        colors,
                        Some(Message::SelectCapture(index))
                    ),
                ]
                .spacing(f32::from(Spacing::XS))
                .align_y(Alignment::Center),
            );
        }

        let capture_list = card(
            "Recordings",
            "🎙",
            column![
                row![
                    self.action_btn(
                        if self.recording.is_some() {
                            "■ Stop recording"
                        } else {
                            "● Record"
                        },
                        if self.recording.is_some() {
                            ButtonKind::Danger
                        } else {
                            ButtonKind::Primary
                        },
                        Message::ToggleRecord
                    ),
                    self.action_btn(
                        if self.punch_in {
                            "Punch-in on"
                        } else {
                            "Punch-in off"
                        },
                        ButtonKind::Ghost,
                        Message::TogglePunchIn
                    ),
                    Space::new().width(Length::Fill),
                    components::button(
                        if self.record_monitor {
                            "Monitor on"
                        } else {
                            "Monitor off"
                        },
                        if self.record_monitor {
                            ButtonKind::Secondary
                        } else {
                            ButtonKind::Ghost
                        },
                        colors,
                        Some(Message::ToggleMonitor)
                    ),
                ]
                .spacing(f32::from(Spacing::XXS))
                .align_y(Alignment::Center),
                scrollable(rows).height(Length::Fill),
            ]
            .spacing(f32::from(Spacing::SM))
            .into(),
            colors,
        );

        let rate = self.session.project().sample_rate.max(1);
        let mut segments = column![].spacing(f32::from(Spacing::XXS));
        if self.transcript.segments.is_empty() {
            segments = segments.push(components::muted(
                "Transcribe a capture to split it into speech regions.",
                colors,
            ));
        }
        for (index, segment) in self.transcript.segments.iter().enumerate() {
            let start = segment.start_frame as f64 / f64::from(rate);
            let end = segment.end_frame as f64 / f64::from(rate);
            let selected = self.selected_segment == Some(index);
            segments = segments.push(
                container(
                    row![
                        text(format!(
                            "{:02}:{:05.2}",
                            (start / 60.0) as u32,
                            start % 60.0
                        ))
                        .size(f32::from(TypeScale::CAPTION))
                        .font(fonts::mono())
                        .color(colors.accent_secondary),
                        text(segment.text.clone())
                            .size(f32::from(TypeScale::BODY))
                            .color(colors.text),
                        Space::new().width(Length::Fill),
                        components::muted(format!("{:.2}s", (end - start).max(0.0)), colors),
                        components::button(
                            "Seek",
                            if selected {
                                ButtonKind::Secondary
                            } else {
                                ButtonKind::Ghost
                            },
                            colors,
                            Some(Message::SelectTranscriptSegment(index))
                        ),
                        components::button(
                            "✕",
                            ButtonKind::Ghost,
                            colors,
                            Some(Message::DeleteTranscriptSegment(index))
                        ),
                    ]
                    .spacing(f32::from(Spacing::XS))
                    .align_y(Alignment::Center),
                )
                .padding([Spacing::XXS, Spacing::XS])
                .style(move |_theme| {
                    if selected {
                        colors.panel_style(true)
                    } else {
                        colors.card_style()
                    }
                }),
            );
        }

        let transcript_card = card(
            "Transcript",
            "≋",
            column![
                row![
                    self.action_btn(
                        "Transcribe capture",
                        ButtonKind::Primary,
                        Message::TranscribeActiveTrack
                    ),
                    self.action_btn(
                        if self.dictating { "Stop dictation" } else { "Dictate" },
                        if self.dictating {
                            ButtonKind::Danger
                        } else {
                            ButtonKind::Secondary
                        },
                        Message::ToggleDictation
                    ),
                    Space::new().width(Length::Fill),
                    self.action_btn("Export SRT", ButtonKind::Ghost, Message::ExportTranscriptSrt),
                    self.action_btn("Export TXT", ButtonKind::Ghost, Message::ExportTranscriptTxt),
                ]
                .spacing(f32::from(Spacing::XXS))
                .align_y(Alignment::Center),
                components::muted(
                    "The built-in segmenter finds speech regions with VAD; it cannot recognise words yet. Segment text is a placeholder until a neural STT model is installed.",
                    colors
                ),
                scrollable(segments).height(Length::Fill),
            ]
            .spacing(f32::from(Spacing::SM))
            .into(),
            colors,
        );

        let input = self
            .record_device
            .clone()
            .unwrap_or_else(|| "Default input".to_owned());
        let capture_card = card(
            "Input",
            "◉",
            column![
                components::dropdown(
                    self.input_devices.clone(),
                    Some(input),
                    Message::SelectInput
                ),
                self.recording_meters_view(colors),
                components::muted(
                    "Recording writes crash-safe 5-second chunks and imports the take as an undoable clip.",
                    colors
                ),
            ]
            .spacing(f32::from(Spacing::SM))
            .into(),
            colors,
        );

        let left = column![capture_list]
            .spacing(f32::from(Spacing::SM))
            .width(Length::FillPortion(2))
            .height(Length::Fill);
        let right = column![transcript_card, capture_card]
            .spacing(f32::from(Spacing::SM))
            .width(Length::FillPortion(3))
            .height(Length::Fill);

        column![
            header(
                "Captures",
                "Recorded takes, input health and text-based edits",
                vec![self.action_btn(
                    "Stories",
                    ButtonKind::Secondary,
                    Message::SelectWorkflowTab(super::WorkflowTab::Stories)
                )],
                colors,
            ),
            row![left, right]
                .spacing(f32::from(Spacing::SM))
                .height(Length::Fill),
        ]
        .spacing(f32::from(Spacing::SM))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    /// Live input meters for the capture card.
    fn recording_meters_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let peak = self.record_peak.clamp(0.0, 1.0);
        let rms = self.record_rms.clamp(0.0, 1.0);
        column![
            row![
                components::kicker("Peak", colors),
                components::progress(peak, colors),
                text(format!("{:.1} dBFS", 20.0 * peak.max(1e-4).log10()))
                    .size(f32::from(TypeScale::CAPTION))
                    .font(fonts::mono())
                    .color(colors.faint),
            ]
            .spacing(f32::from(Spacing::XS))
            .align_y(Alignment::Center),
            row![
                components::kicker("RMS", colors),
                components::progress(rms, colors),
                text(format!("{:.1} dBFS", 20.0 * rms.max(1e-4).log10()))
                    .size(f32::from(TypeScale::CAPTION))
                    .font(fonts::mono())
                    .color(colors.faint),
            ]
            .spacing(f32::from(Spacing::XS))
            .align_y(Alignment::Center),
            components::muted(
                if self.recording.is_some() {
                    "● recording"
                } else {
                    "idle"
                },
                colors
            ),
        ]
        .spacing(f32::from(Spacing::XS))
        .into()
    }
}
