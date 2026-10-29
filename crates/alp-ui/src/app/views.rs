//! Screen and panel views for the shell.
//!
//! Views are pure: they read [`App`](super::App) state and return iced elements.
//! They never mutate the project or the panel model directly.

use alp_ui_kit::icons;
use iced::widget::{Space, column, container, mouse_area, row, stack, text};

use super::*;

impl App {
    pub(super) fn action_btn<'a>(
        &self,
        label: impl Into<String>,
        kind: ButtonKind,
        msg: Message,
    ) -> Element<'a, Message> {
        components::button(label, kind, Some(msg))
    }

    /// Render the whole shell.
    pub fn view(&self) -> Element<'_, Message> {
        let colors = self.colors();
        let mut layers: Vec<Element<'_, Message>> = Vec::new();

        let workspace = container(self.workspace_view(colors))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding([Spacing::XL, Control::PAGE]);

        let frame = container(
            column![
                self.chrome_view(colors),
                row![
                    self.sidebar_view(colors),
                    column![workspace, self.dock_view(colors)]
                        .width(Length::Fill)
                        .height(Length::Fill),
                ]
                .spacing(0.0)
                .width(Length::Fill)
                .height(Length::Fill),
            ]
            .spacing(0.0),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_theme| colors.frame_style());

        let backdrop = container(frame)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(Spacing::SM)
            .style(move |_theme| container::Style {
                background: Some(Background::Color(colors.background)),
                ..container::Style::default()
            });

        layers.push(backdrop.into());

        if self.palette.open {
            let items: Vec<CommandItem<Message>> = self
                .palette
                .visible
                .iter()
                .filter_map(|index| self.commands.get(*index))
                .map(|entry| CommandItem {
                    title: entry.title.to_string(),
                    hint: if !entry.hint.is_empty() {
                        format!("{} · {}", entry.category, entry.hint)
                    } else if let Some(shortcut) = commands::shortcut(entry.id) {
                        shortcut.label()
                    } else {
                        entry.category.to_string()
                    },
                    message: Message::RunCommand(entry.id),
                })
                .collect();
            layers.push(overlays::command_palette(
                self.palette_id.clone(),
                &self.palette.query,
                &items,
                self.palette.selected,
                colors,
                Message::PaletteInput,
            ));
        }

        if let Some((message, _)) = &self.toast {
            layers.push(
                container(overlays::toast(
                    message.clone(),
                    colors,
                    Message::DismissToast,
                ))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(iced::alignment::Vertical::Bottom)
                .padding([Spacing::XXL, Spacing::XXL])
                .into(),
            );
        }

        if self.shortcuts_help_open {
            layers.push(self.shortcuts_help_modal_view(colors));
        }

        if self.create_voice_open {
            layers.push(self.create_voice_dialog_view(colors));
        }

        if self.consent_dialog_open {
            layers.push(self.consent_modal_view(colors));
        }

        if self.first_run_open {
            layers.push(self.first_run_wizard_view(colors));
        }

        stack(layers)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    pub(super) fn chrome_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let drag = mouse_area(
            row![
                text("Audio Labs Pro")
                    .size(f32::from(TypeScale::CAPTION))
                    .font(fonts::semibold())
                    .color(colors.muted),
                text("· Studio")
                    .size(f32::from(TypeScale::CAPTION))
                    .font(fonts::regular())
                    .color(colors.faint),
            ]
            .spacing(f32::from(Spacing::XS))
            .align_y(Alignment::Center),
        )
        .on_press(Message::WindowDrag);

        let chrome = row![
            self.traffic_lights(colors),
            Space::new().width(Length::Fixed(f32::from(Spacing::MD))),
            drag,
            Space::new().width(Length::Fill),
            components::with_tooltip(
                components::glyph_button(
                    if self.reduce_motion {
                        icons::PAUSE
                    } else {
                        icons::ACTIVITY
                    },
                    13.0,
                    ButtonKind::Ghost,
                    Some(Message::ToggleReduceMotion)
                ),
                "Reduce motion".to_string(),
                colors,
            ),
            components::with_tooltip(
                components::glyph_button(
                    icons::THEME,
                    14.0,
                    ButtonKind::Ghost,
                    Some(Message::ToggleTheme)
                ),
                self.theme_mode.as_str().to_string(),
                colors,
            ),
            components::with_tooltip(
                components::glyph_button(
                    icons::KEYBOARD,
                    13.0,
                    ButtonKind::Ghost,
                    Some(Message::OpenShortcutsHelp)
                ),
                "Keyboard shortcuts".to_string(),
                colors,
            ),
        ]
        .spacing(f32::from(Spacing::XXS))
        .align_y(Alignment::Center);

        container(chrome)
            .width(Length::Fill)
            .height(Length::Fixed(f32::from(Control::TITLE_BAR)))
            .padding([0.0, f32::from(Spacing::MD)])
            .style(move |_theme| container::Style {
                background: Some(Background::Color(colors.glass)),
                border: Border {
                    radius: 0.0.into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
                shadow: iced::Shadow::default(),
                text_color: Some(colors.text),
                snap: true,
            })
            .into()
    }

    pub(super) fn mixer_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let mut rows = column![]
            .spacing(f32::from(Spacing::SM))
            .push(components::kicker("Tracks", colors));
        for (index, track) in self.session.tracks().iter().enumerate() {
            let meter = self.live_track_peak(index);
            let gain = track.gain;
            let pan = track.pan;
            let strip = column![
                row![
                    text(track.name.clone())
                        .size(f32::from(TypeScale::LABEL))
                        .font(fonts::semibold())
                        .color(colors.text),
                    Space::new().width(Length::Fill),
                    components::toggle("M", track.muted, Message::ToggleTrackMute(index)),
                    components::toggle("S", track.solo, Message::ToggleTrackSolo(index)),
                    components::toggle("R", track.armed, Message::ToggleTrackArm(index)),
                ]
                .spacing(f32::from(Spacing::XXS))
                .align_y(Alignment::Center),
                row![
                    components::muted(format!("trim {:.0}%", pan * 100.0), colors),
                    components::progress(meter, colors),
                    components::slider_control(gain / 2.0, move |value| {
                        Message::SetTrackGain(index, value * 2.0)
                    }),
                ]
                .spacing(f32::from(Spacing::XS))
                .align_y(Alignment::Center),
                overlays::property_row(
                    "Pan",
                    components::slider_control((pan + 1.0) / 2.0, move |value| {
                        Message::SetTrackPan(index, value * 2.0 - 1.0)
                    }),
                    colors,
                ),
            ]
            .spacing(f32::from(Spacing::XS))
            .padding(Spacing::SM);
            rows = rows.push(
                container(strip)
                    .width(Length::Fill)
                    .style(move |_theme| colors.well_style()),
            );
        }
        let master = column![
            components::kicker("Master", colors),
            overlays::property_row(
                "Bus",
                components::slider_control(self.gain / 4.0, |value| Message::SetMasterGain(
                    value * 4.0
                )),
                colors,
            ),
            components::toggle("Ripple delete", self.ripple, Message::ToggleRipple),
            components::progress(self.record_peak, colors),
        ]
        .spacing(f32::from(Spacing::XS))
        .padding(Spacing::SM);

        let content = column![
            rows,
            container(master)
                .width(Length::Fill)
                .style(move |_theme| colors.well_style()),
        ]
        .spacing(f32::from(Spacing::SM))
        .padding(f32::from(Spacing::SM))
        .width(Length::Fill)
        .height(Length::Fill);

        self.scroll(content, colors).into()
    }

    pub(super) fn timeline_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let x = self.waveform.visible();
        let waveform = container(self.waveform.view())
            .width(Length::Fill)
            .height(Length::Fixed(120.0))
            .style(move |_theme| colors.card_style());

        let mut tracks = column![].spacing(f32::from(Spacing::XXS));
        for (index, track) in self.session.tracks().iter().enumerate() {
            let selected = index == self.selected_track;
            let name = &track.name;
            let clips_count = track.clips.len();
            let row = components::button(
                format!("{index:02}  {name} ({clips_count} clips)"),
                if selected {
                    ButtonKind::Secondary
                } else {
                    ButtonKind::Ghost
                },
                Some(Message::SelectTrack(index)),
            );
            tracks = tracks.push(row);
        }
        if !self.session.markers().is_empty() {
            let marker_text = self
                .session
                .markers()
                .iter()
                .map(|m| {
                    format!(
                        "{} ({:.1}s)",
                        m.label,
                        frames_to_secs(m.frame, self.session.project().sample_rate)
                    )
                })
                .collect::<Vec<_>>()
                .join(" · ");
            tracks = tracks.push(components::muted(format!("Markers: {marker_text}"), colors));
        }
        if let Some(loop_range) = self.session.loop_range() {
            let loop_text = format!(
                "Loop: {:.2}s → {:.2}s",
                frames_to_secs(loop_range.start, self.session.project().sample_rate),
                frames_to_secs(loop_range.end, self.session.project().sample_rate),
            );
            tracks = tracks.push(components::muted(loop_text, colors));
        }

        column![
            row![
                components::icon_button(icons::MINUS, Some(Message::ZoomWaveform(1.25))),
                components::icon_button(icons::PLUS, Some(Message::ZoomWaveform(0.8))),
                components::icon_button(icons::RESET, Some(Message::SeekWaveform(0.0))),
                components::muted(format!("view {:.1}s → {:.1}s", x.0, x.1), colors),
                Space::new().width(Length::Fill),
                components::muted("Scroll over waveform to zoom", colors),
            ]
            .spacing(f32::from(Spacing::XS))
            .align_y(Alignment::Center),
            waveform,
            self.scroll(tracks, colors).height(Length::Fill),
        ]
        .spacing(f32::from(Spacing::SM))
        .into()
    }

    pub(super) fn inspector_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let mut clips = column![].spacing(f32::from(Spacing::XXS));
        if let Some(track) = self.session.project().track(self.session.track_id()) {
            for clip in &track.clips {
                let selected = self.session.selected_clip() == Some(clip.id);
                let start = frames_to_secs(clip.timeline_start, self.session.project().sample_rate);
                let end = frames_to_secs(
                    self.session.project().clip_timeline_end(clip),
                    self.session.project().sample_rate,
                );
                clips = clips.push(components::button(
                    format!("{start:.2}s → {end:.2}s · {:.0}%", clip.gain * 100.0),
                    if selected {
                        ButtonKind::Secondary
                    } else {
                        ButtonKind::Ghost
                    },
                    Some(Message::SelectClip(clip.id)),
                ));
            }
        }
        let undo = self.session.can_undo().then_some(Message::UndoEdit);
        let redo = self.session.can_redo().then_some(Message::RedoEdit);
        self.scroll(
            column![
                components::kicker("Track edit", colors),
                components::muted(self.session.status(), colors),
                row![
                    components::button("Undo", ButtonKind::Ghost, undo),
                    components::button("Redo", ButtonKind::Ghost, redo),
                    components::muted(
                        format!(
                            "{} clips · {:.1}s",
                            self.session.clip_count(),
                            self.session.duration_secs()
                        ),
                        colors
                    ),
                ]
                .spacing(f32::from(Spacing::XS))
                .align_y(Alignment::Center),
                components::kicker("Clips", colors),
                self.scroll(clips, colors).height(Length::Fixed(112.0)),
                components::kicker("Selection (seconds)", colors),
                row![
                    components::text_field(
                        "start",
                        &self.selection_start,
                        colors,
                        Message::SelectionStart
                    ),
                    components::text_field(
                        "end",
                        &self.selection_end,
                        colors,
                        Message::SelectionEnd
                    ),
                    components::button(
                        "Apply",
                        ButtonKind::Secondary,
                        Some(Message::ApplySelection)
                    ),
                ]
                .spacing(f32::from(Spacing::XS)),
                // The operation stack, one labelled group per undoable family.
                components::kicker("Clipboard", colors),
                row![
                    self.action_btn("Cut", ButtonKind::Ghost, Message::CutSelection),
                    self.action_btn("Copy", ButtonKind::Ghost, Message::CopySelection),
                    self.action_btn("Paste", ButtonKind::Ghost, Message::PasteClipboard),
                    self.action_btn("Delete", ButtonKind::Danger, Message::DeleteSelection),
                ]
                .spacing(f32::from(Spacing::XXS)),
                components::kicker("Edit", colors),
                row![
                    self.action_btn("Silence", ButtonKind::Ghost, Message::SilenceSelection),
                    self.action_btn("Trim", ButtonKind::Ghost, Message::TrimSelection),
                    self.action_btn("Split", ButtonKind::Ghost, Message::SplitSelection),
                    self.action_btn(
                        "XFade 0.5s",
                        ButtonKind::Ghost,
                        Message::CrossfadeSelection(0.5)
                    ),
                    self.action_btn(
                        "XFade 1.0s",
                        ButtonKind::Ghost,
                        Message::CrossfadeSelection(1.0)
                    ),
                    self.action_btn("Duplicate", ButtonKind::Ghost, Message::DuplicateSelection),
                ]
                .spacing(f32::from(Spacing::XXS)),
                components::kicker("Arrange", colors),
                row![
                    self.action_btn(
                        "Move up a track",
                        ButtonKind::Ghost,
                        Message::MoveSelectedClipToPrevTrack
                    ),
                    self.action_btn(
                        "Move down a track",
                        ButtonKind::Ghost,
                        Message::MoveSelectedClipToNextTrack
                    ),
                ]
                .spacing(f32::from(Spacing::XXS)),
                components::kicker("Process", colors),
                row![
                    self.action_btn("Normalize", ButtonKind::Ghost, Message::NormalizeSelection),
                    self.action_btn("Reverse", ButtonKind::Ghost, Message::ToggleReverse),
                    self.action_btn("Invert", ButtonKind::Ghost, Message::ToggleInvert),
                ]
                .spacing(f32::from(Spacing::XXS)),
                components::kicker("Envelope", colors),
                row![
                    self.action_btn("Dip envelope", ButtonKind::Ghost, Message::DipEnvelope),
                    self.action_btn("Flat envelope", ButtonKind::Ghost, Message::ResetEnvelope),
                ]
                .spacing(f32::from(Spacing::XXS)),
                overlays::property_row(
                    "Gain",
                    components::slider_control(self.edit_gain / 2.0, |value| {
                        Message::EditGain(value * 2.0)
                    }),
                    colors
                ),
                overlays::property_row(
                    "Fade in",
                    components::slider_control(self.edit_fade_in, Message::EditFadeIn),
                    colors
                ),
                overlays::property_row(
                    "Fade out",
                    components::slider_control(self.edit_fade_out, Message::EditFadeOut),
                    colors
                ),
            ]
            .spacing(f32::from(Spacing::SM)),
            colors,
        )
        .height(Length::Fill)
        .into()
    }

    pub(super) fn effects_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let mut rows = column![].spacing(f32::from(Spacing::XS));
        for (index, eff) in self.effects.iter().enumerate() {
            let selected = index == self.selected_effect;
            let body = row![
                text(format!("{:02}", index + 1))
                    .size(f32::from(TypeScale::CAPTION))
                    .font(fonts::mono())
                    .color(if selected {
                        colors.accent
                    } else {
                        colors.faint
                    }),
                column![
                    text(eff.name.clone())
                        .size(f32::from(TypeScale::BODY_STRONG))
                        .font(fonts::semibold())
                        .color(colors.text),
                    components::muted(eff.params.clone(), colors),
                ]
                .spacing(f32::from(Spacing::XXS)),
                Space::new().width(Length::Fill),
                components::switch(eff.enabled, Message::ToggleEffect(index)),
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
                            radius: f32::from(alp_ui_kit::Radius::SM).into(),
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
                }))
                .on_press(Message::SelectEffect(index)),
            );
        }

        let detail = self.effects.get(self.selected_effect).map_or_else(
            || "Select an effect stage to inspect parameters".into(),
            |e| {
                let s = if e.enabled { "active" } else { "bypassed" };
                format!("{}: {} · {s}", e.name, e.params)
            },
        );

        column![
            rows,
            components::muted(detail, colors),
            components::muted("Master target -16.0 LUFS · ceiling -1.0 dBFS", colors),
        ]
        .spacing(f32::from(Spacing::SM))
        .into()
    }

    /// Discovered CLAP/LV2 plugins with an insert action per row.
    pub(super) fn plugin_list_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        if self.discovered_plugins.is_empty() {
            return components::muted(
                "No plugins discovered. Set CLAP_PATH or LV2_PATH and rescan.",
                colors,
            );
        }
        let mut plugin_rows = column![].spacing(f32::from(Spacing::XS));
        for p in self.discovered_plugins.iter().take(8) {
            let fmt_str = match p.format {
                alp_plugin::PluginFormat::Clap => "CLAP",
                alp_plugin::PluginFormat::Lv2 => "LV2",
            };
            let body = row![
                components::chip(fmt_str, colors),
                text(p.name.clone())
                    .size(f32::from(TypeScale::BODY))
                    .font(fonts::regular())
                    .color(colors.text),
                Space::new().width(Length::Fill),
                components::button_icon(
                    "Insert",
                    icons::PLUS,
                    ButtonKind::Secondary,
                    Some(Message::InsertPlugin(p.id.clone()))
                ),
            ]
            .align_y(Alignment::Center)
            .spacing(f32::from(Spacing::SM))
            .padding([Spacing::XS, Spacing::SM]);

            plugin_rows = plugin_rows.push(
                container(body)
                    .width(Length::Fill)
                    .style(move |_theme| colors.well_style()),
            );
        }
        self.scroll(plugin_rows, colors).height(Length::Fill).into()
    }

    pub(super) fn export_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let mut rows = column![].spacing(f32::from(Spacing::SM));
        for job in self.jobs.jobs() {
            let status = match job.status {
                JobStatus::Running => format!("{:.0}%", job.progress * 100.0),
                JobStatus::Completed => "Done".into(),
                JobStatus::Cancelled => "Cancelled".into(),
                JobStatus::Failed => "Failed".into(),
            };
            let cancel_btn = if job.status.is_running() {
                components::button(
                    "Cancel",
                    ButtonKind::Ghost,
                    Some(Message::CancelJob(job.id)),
                )
            } else {
                Space::new().width(Length::Fixed(0.0)).into()
            };
            rows = rows.push(
                container(
                    column![
                        row![
                            text(job.name.clone())
                                .size(f32::from(TypeScale::BODY_STRONG))
                                .font(fonts::semibold())
                                .color(colors.text),
                            Space::new().width(Length::Fill),
                            components::muted(status, colors),
                            cancel_btn,
                        ]
                        .align_y(Alignment::Center)
                        .spacing(f32::from(Spacing::XS)),
                        components::progress(job.progress, colors),
                        components::muted(job.detail.clone(), colors),
                    ]
                    .spacing(f32::from(Spacing::XS))
                    .padding(Spacing::MD),
                )
                .width(Length::Fill)
                .style(move |_theme| colors.well_style()),
            );
        }
        let running_label = if self.jobs.running_count() > 0 {
            format!("{} running", self.jobs.running_count())
        } else {
            "idle".into()
        };
        let status_label = self.session.last_export().unwrap_or_else(|| {
            if self.jobs.has_running() {
                "A worker is running; the UI stays responsive."
            } else {
                "24-bit WAV · background render"
            }
        });
        let queue: Element<'_, Message> = if self.jobs.jobs().is_empty() {
            components::muted("No export jobs yet.", colors)
        } else {
            self.scroll(rows, colors)
                .height(Length::Fixed(150.0))
                .into()
        };
        column![
            row![
                self.action_btn("Export mix", ButtonKind::Primary, Message::ExportMix),
                self.action_btn("Clear done", ButtonKind::Ghost, Message::ClearJobs),
                Space::new().width(Length::Fill),
                components::muted(running_label, colors),
            ]
            .spacing(f32::from(Spacing::XS))
            .align_y(Alignment::Center),
            queue,
            components::muted(status_label, colors),
        ]
        .spacing(f32::from(Spacing::SM))
        .into()
    }
}
