//! Captures workspace: recordings, input health and transcript editing.

use alp_ui_kit::components;
use alp_ui_kit::icons;
use alp_ui_kit::tokens::{Radius, Spacing, TypeScale};
use alp_ui_kit::{ButtonKind, ThemeColors, fonts};
use iced::widget::{Space, column, container, mouse_area, row, text};
use iced::{Alignment, Background, Border, Element, Length};

use super::workspace_layout::{card, card_auto, header};
use super::{App, Message, format_time, shorten};

impl App {
    /// Captures workspace: record, audit levels and edit the transcript.
    pub(super) fn captures_workspace(&self, colors: ThemeColors) -> Element<'_, Message> {
        let capture_list = self.capture_list_view(colors);
        let transcript_card = self.transcript_view(colors);
        let input_card = self.input_view(colors);

        let left = column![capture_list]
            .spacing(f32::from(Spacing::SM))
            .width(Length::FillPortion(2))
            .height(Length::Fill);
        let right = column![transcript_card, input_card]
            .spacing(f32::from(Spacing::LG))
            .width(Length::FillPortion(3))
            .height(Length::Fill);

        column![
            header(
                "Captures",
                "Recorded takes, input health and text-based edits",
                vec![
                    components::button(
                        "Open Stories",
                        ButtonKind::Secondary,
                        Some(Message::SelectWorkflowTab(super::WorkflowTab::Stories))
                    ),
                    components::button_icon(
                        if self.recording.is_some() {
                            "Stop"
                        } else {
                            "Record"
                        },
                        if self.recording.is_some() {
                            icons::STOP
                        } else {
                            icons::RECORD
                        },
                        if self.recording.is_some() {
                            ButtonKind::Danger
                        } else {
                            ButtonKind::Primary
                        },
                        Some(Message::ToggleRecord)
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
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn capture_list_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let mut rows = column![].spacing(f32::from(Spacing::XS));
        if self.captures.is_empty() {
            rows = rows.push(
                container(
                    column![
                        text("No captures yet")
                            .size(f32::from(TypeScale::HEADING))
                            .font(fonts::semibold())
                            .color(colors.text),
                        components::muted(
                            "Press Record. Takes are chunked every five seconds and imported as one undoable clip.",
                            colors,
                        ),
                    ]
                    .spacing(f32::from(Spacing::XXS)),
                )
                .padding(Spacing::LG)
                .width(Length::Fill)
                .style(move |_theme| colors.well_style()),
            );
        }
        for (index, capture) in self.captures.iter().enumerate() {
            let selected = index == self.selected_capture;
            let body =
                row![
                    container(text(if selected { "●" } else { "○" }).size(10.0).color(
                        if selected {
                            colors.accent
                        } else {
                            colors.faint
                        }
                    ),)
                    .width(Length::Fixed(16.0))
                    .align_x(Alignment::Center),
                    column![
                        text(shorten(&capture.name, 48))
                            .size(f32::from(TypeScale::BODY_STRONG))
                            .font(fonts::semibold())
                            .color(colors.text),
                        row![
                            components::chip_mono(format_time(capture.duration_secs), colors),
                            components::chip(format!("peak {:.0}%", capture.peak * 100.0), colors),
                        ]
                        .spacing(f32::from(Spacing::XXS)),
                    ]
                    .spacing(f32::from(Spacing::XS)),
                    Space::new().width(Length::Fill),
                    components::with_tooltip(
                        components::glyph_button(
                            icons::GRID,
                            13.0,
                            if selected {
                                ButtonKind::Secondary
                            } else {
                                ButtonKind::Ghost
                            },
                            Some(Message::SelectCapture(index))
                        ),
                        "Select this capture".to_string(),
                        colors,
                    ),
                ]
                .spacing(f32::from(Spacing::MD))
                .align_y(Alignment::Center)
                .padding(Spacing::MD);

            rows = rows.push(
                mouse_area(container(body).width(Length::Fill).style(move |_theme| {
                    container::Style {
                        background: Some(Background::Color(if selected {
                            colors.surface_raised
                        } else {
                            colors.glass_strong
                        })),
                        border: Border {
                            radius: f32::from(Radius::MD).into(),
                            width: 1.0,
                            color: if selected {
                                colors.accent
                            } else {
                                colors.border
                            },
                        },
                        shadow: iced::Shadow::default(),
                        text_color: Some(colors.text),
                        snap: true,
                    }
                }))
                .on_press(Message::SelectCapture(index)),
            );
        }

        let controls = row![
            components::button(
                if self.punch_in {
                    "Punch-in on"
                } else {
                    "Punch-in off"
                },
                if self.punch_in {
                    ButtonKind::Secondary
                } else {
                    ButtonKind::Ghost
                },
                Some(Message::TogglePunchIn)
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
                Some(Message::ToggleMonitor)
            ),
        ]
        .spacing(f32::from(Spacing::XS))
        .align_y(Alignment::Center);

        card(
            "Recordings",
            icons::CAPTURES,
            column![controls, self.scroll(rows, colors).height(Length::Fill)]
                .spacing(f32::from(Spacing::MD))
                .into(),
            colors,
        )
    }

    fn transcript_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let rate = self.session.project().sample_rate.max(1);
        let mut segments = column![].spacing(f32::from(Spacing::XS));
        if self.transcript.segments.is_empty() {
            segments = segments.push(
                container(
                    column![
                        text("Nothing transcribed yet")
                            .size(f32::from(TypeScale::BODY_STRONG))
                            .font(fonts::semibold())
                            .color(colors.text),
                        components::muted(
                            "Transcribe a capture to split it into speech regions you can seek and trim.",
                            colors,
                        ),
                    ]
                    .spacing(f32::from(Spacing::XXS)),
                )
                .padding(Spacing::LG)
                .width(Length::Fill)
                .style(move |_theme| colors.well_style()),
            );
        }
        for (index, segment) in self.transcript.segments.iter().enumerate() {
            let start = segment.start_frame as f64 / f64::from(rate);
            let end = segment.end_frame as f64 / f64::from(rate);
            let selected = self.selected_segment == Some(index);
            let body = row![
                text(format!(
                    "{:02}:{:05.2}",
                    (start / 60.0) as u32,
                    start % 60.0
                ))
                .size(f32::from(TypeScale::CAPTION))
                .font(fonts::mono())
                .color(colors.accent),
                text(segment.text.clone())
                    .size(f32::from(TypeScale::BODY))
                    .font(fonts::regular())
                    .color(colors.text),
                Space::new().width(Length::Fill),
                components::chip_mono(format!("{:.2}s", (end - start).max(0.0)), colors),
                components::with_tooltip(
                    components::glyph_button(
                        icons::PLAY,
                        12.0,
                        ButtonKind::Ghost,
                        Some(Message::SelectTranscriptSegment(index))
                    ),
                    "Seek here".to_string(),
                    colors,
                ),
                components::with_tooltip(
                    components::glyph_button(
                        icons::DELETE_KEY,
                        12.0,
                        ButtonKind::Ghost,
                        Some(Message::DeleteTranscriptSegment(index))
                    ),
                    "Remove segment".to_string(),
                    colors,
                ),
            ]
            .spacing(f32::from(Spacing::SM))
            .align_y(Alignment::Center)
            .padding([Spacing::SM, Spacing::MD]);

            segments = segments.push(container(body).width(Length::Fill).style(move |_theme| {
                container::Style {
                    background: Some(Background::Color(if selected {
                        colors.surface_raised
                    } else {
                        colors.glass_strong
                    })),
                    border: Border {
                        radius: f32::from(Radius::SM).into(),
                        width: 1.0,
                        color: if selected {
                            colors.border_strong
                        } else {
                            colors.border
                        },
                    },
                    shadow: iced::Shadow::default(),
                    text_color: Some(colors.text),
                    snap: true,
                }
            }));
        }

        let toolbar = row![
            components::button(
                "Transcribe capture",
                ButtonKind::Primary,
                Some(Message::TranscribeActiveTrack)
            ),
            components::button(
                if self.dictating {
                    "Stop dictation"
                } else {
                    "Dictate"
                },
                if self.dictating {
                    ButtonKind::Danger
                } else {
                    ButtonKind::Secondary
                },
                Some(Message::ToggleDictation)
            ),
            Space::new().width(Length::Fill),
            components::button(
                "Export SRT",
                ButtonKind::Ghost,
                Some(Message::ExportTranscriptSrt)
            ),
            components::button(
                "Export TXT",
                ButtonKind::Ghost,
                Some(Message::ExportTranscriptTxt)
            ),
        ]
        .spacing(f32::from(Spacing::XS))
        .align_y(Alignment::Center);

        card(
            "Transcript",
            icons::FILE_TEXT,
            column![
                toolbar,
                components::muted(
                    "The built-in segmenter finds speech regions with VAD; it cannot recognise words yet. Segment text is a placeholder until a neural STT model is installed.",
                    colors
                ),
                self.scroll(segments, colors).height(Length::Fill),
            ]
            .spacing(f32::from(Spacing::SM))
            .into(),
            colors,
        )
    }

    fn input_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let input = self
            .record_device
            .clone()
            .unwrap_or_else(|| "Default input".to_owned());
        card_auto(
            "Input",
            icons::MIC,
            column![
                components::field_label("Recording device", colors),
                components::dropdown(
                    self.input_devices.clone(),
                    Some(input),
                    colors,
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
        )
    }

    /// Live input meters for the capture card.
    fn recording_meters_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let peak = self.record_peak.clamp(0.0, 1.0);
        let rms = self.record_rms.clamp(0.0, 1.0);
        let meter = |label: &'static str, value: f32| {
            row![
                components::kicker(label, colors),
                container(components::progress(value, colors)).width(Length::Fixed(150.0)),
                Space::new().width(Length::Fill),
                text(format!("{:.1} dBFS", 20.0 * value.max(1e-4).log10()))
                    .size(f32::from(TypeScale::CAPTION))
                    .font(fonts::mono())
                    .color(colors.faint),
            ]
            .spacing(f32::from(Spacing::SM))
            .align_y(Alignment::Center)
        };
        column![
            meter("Peak", peak),
            meter("RMS", rms),
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
