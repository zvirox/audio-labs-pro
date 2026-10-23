//! Screen and panel views for the shell.
//!
//! Views are pure: they read [`App`](super::App) state and return iced elements.
//! They never mutate the project or the panel model directly.

use iced::widget::{
    Space, column, container, mouse_area, row, scrollable, stack, text, text_input,
};

use super::*;

impl App {
    /// Render the whole shell.
    pub fn view(&self) -> Element<'_, Message> {
        let colors = self.colors();
        let mut layers: Vec<Element<'_, Message>> = Vec::new();
        layers.push(
            container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_theme| container::Style {
                    background: Some(Background::Color(Color {
                        a: 0.58,
                        ..colors.background
                    })),
                    ..container::Style::default()
                })
                .into(),
        );

        for kind in self.visible_panels() {
            layers.push(self.panel_view(kind, colors));
        }

        layers.push(self.chrome_view(colors));

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
                .padding(Spacing::XXL)
                .into(),
            );
        }

        stack(layers)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn visible_panels(&self) -> Vec<PanelKind> {
        let mut kinds = self.panels.painter_order();
        for panel in self.panels.panels() {
            if !panel.open
                && self
                    .panel_motion
                    .get(&panel.kind)
                    .is_some_and(|toggle| toggle.value() > 0.01)
            {
                kinds.push(panel.kind);
            }
        }
        kinds
    }

    fn chrome_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let drag = mouse_area(
            row![
                text("◈").size(15.0).color(colors.accent),
                text("AUDIO LAB PRO")
                    .size(f32::from(TypeScale::CAPTION))
                    .font(fonts::semibold())
                    .color(colors.text),
                components::muted(
                    format!(
                        "{:.0} fps · {:.1} ms",
                        1000.0 / self.frame_ms.max(0.1),
                        self.frame_ms
                    ),
                    colors,
                ),
            ]
            .spacing(f32::from(Spacing::SM))
            .align_y(Alignment::Center),
        )
        .on_press(Message::WindowDrag);

        let mut panel_buttons = row![].spacing(f32::from(Spacing::XXS));
        for kind in PanelKind::ALL {
            panel_buttons = panel_buttons.push(components::with_tooltip(
                components::icon_button(kind.glyph(), colors, Some(Message::TogglePanel(kind))),
                kind.id(),
                colors,
            ));
        }

        let chrome = row![
            drag,
            Space::new().width(Length::Fill),
            panel_buttons,
            components::with_tooltip(
                components::button(self.glass.badge(), ButtonKind::Ghost, colors, None),
                self.glass.detail.clone(),
                colors,
            ),
            components::icon_button("◐", colors, Some(Message::ToggleTheme)),
            components::icon_button("≈", colors, Some(Message::ToggleReduceMotion)),
            components::icon_button("⌄", colors, Some(Message::WindowMinimize)),
            components::icon_button("□", colors, Some(Message::WindowMaximize)),
            components::icon_button("×", colors, Some(Message::WindowClose)),
        ]
        .spacing(f32::from(Spacing::XXS))
        .align_y(Alignment::Center);

        container(chrome)
            .width(Length::Fill)
            .height(Length::Fixed(f32::from(Control::CHROME)))
            .padding([Spacing::XS, Spacing::SM])
            .style(move |_theme| container::Style {
                background: Some(Background::Color(colors.glass_strong)),
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

    fn panel_view(&self, kind: PanelKind, colors: ThemeColors) -> Element<'_, Message> {
        let panel = self.panels.panel(kind);
        let motion = self.panel_motion.get(&kind).map_or(
            PanelMotion {
                open: 1.0,
                offset: 0.0,
            },
            PanelMotion::from_open,
        );
        let focused = self.panels.active() == Some(kind);
        let offset = motion.offset;
        let height = if panel.collapsed {
            f32::from(Control::TITLE_BAR)
        } else {
            panel.rect.height
        };

        let drag = mouse_area(
            row![
                text(kind.glyph()).size(12.0).color(colors.faint),
                text(kind.title())
                    .size(f32::from(TypeScale::BODY_STRONG))
                    .font(fonts::semibold())
                    .color(colors.text),
            ]
            .spacing(f32::from(Spacing::XS))
            .align_y(Alignment::Center)
            .width(Length::Fill),
        )
        .on_press(Message::DragPanel(kind));

        let dock_glyph = if panel.docked.is_some() { "⇤" } else { "⇥" };
        let header = row![
            drag,
            components::icon_button(dock_glyph, colors, Some(Message::ToggleDock(kind))),
            components::icon_button(
                if panel.collapsed { "⌄" } else { "⌃" },
                colors,
                Some(Message::CollapsePanel(kind))
            ),
            components::icon_button("×", colors, Some(Message::ClosePanel(kind))),
        ]
        .spacing(f32::from(Spacing::XXS))
        .align_y(Alignment::Center)
        .padding([Spacing::XXS, Spacing::SM]);

        let body: Element<'_, Message> = if panel.collapsed {
            Space::new().height(Length::Fixed(0.0)).into()
        } else {
            container(self.panel_body(kind, colors))
                .padding([Spacing::XS, Spacing::MD])
                .height(Length::Fill)
                .clip(true)
                .into()
        };

        let panel_container = container(column![header, body].spacing(0.0))
            .width(Length::Fixed(panel.rect.width))
            .height(Length::Fixed(height))
            .style(move |_theme| colors.panel_style(focused));

        let content: Element<'_, Message> = if panel.collapsed || panel.docked.is_some() {
            panel_container.into()
        } else {
            stack![
                panel_container,
                container(
                    mouse_area(
                        container(text("◢").size(10.0).color(colors.faint))
                            .width(Length::Fixed(f32::from(Control::GRIP)))
                            .height(Length::Fixed(f32::from(Control::GRIP)))
                            .align_x(Alignment::Center)
                            .align_y(Alignment::Center),
                    )
                    .on_press(Message::ResizePanel(kind)),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::End)
                .align_y(Alignment::End)
                .padding(Spacing::XXS),
            ]
            .into()
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(Padding {
                top: panel.rect.y + offset,
                left: panel.rect.x,
                right: 0.0,
                bottom: 0.0,
            })
            .into()
    }

    fn panel_body(&self, kind: PanelKind, colors: ThemeColors) -> Element<'_, Message> {
        match kind {
            PanelKind::Transport => self.transport_view(colors),
            PanelKind::Mixer => self.mixer_view(colors),
            PanelKind::Timeline => self.timeline_view(colors),
            PanelKind::Inspector => self.inspector_view(colors),
            PanelKind::MediaBin => self.media_view(colors),
            PanelKind::VoiceLab => self.voice_view(colors),
            PanelKind::Effects => self.effects_view(colors),
            PanelKind::Models => self.models_view(colors),
            PanelKind::ExportQueue => self.export_view(colors),
        }
    }

    fn mixer_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let mut rows = column!().spacing(f32::from(Spacing::XS)).push(
            text("TRACKS")
                .size(f32::from(TypeScale::CAPTION))
                .color(colors.text),
        );
        for (index, track) in self.session.tracks().iter().enumerate() {
            let meter = self.record_peak;
            let gain = track.gain;
            let pan = track.pan;
            rows = rows.push(
                column![
                    row![
                        text(track.name.clone())
                            .size(f32::from(TypeScale::CAPTION))
                            .font(fonts::semibold())
                            .color(colors.text),
                        Space::new().width(Length::Fill),
                        components::toggle(
                            "M",
                            track.muted,
                            colors,
                            Message::ToggleTrackMute(index)
                        ),
                        components::toggle(
                            "S",
                            track.solo,
                            colors,
                            Message::ToggleTrackSolo(index)
                        ),
                        components::toggle(
                            "R",
                            track.armed,
                            colors,
                            Message::ToggleTrackArm(index)
                        ),
                    ]
                    .spacing(f32::from(Spacing::XXS)),
                    row![
                        components::muted(format!("trim {:.0}%", pan * 100.0), colors),
                        components::progress(meter, colors),
                        components::slider_control(gain / 2.0, move |value| {
                            Message::SetTrackGain(index, value * 2.0)
                        }),
                    ]
                    .spacing(f32::from(Spacing::XXS)),
                    overlays::property_row(
                        "Pan",
                        components::slider_control((pan + 1.0) / 2.0, move |value| {
                            Message::SetTrackPan(index, value * 2.0 - 1.0)
                        }),
                        colors,
                    ),
                ]
                .spacing(f32::from(Spacing::XXS)),
            );
        }
        let master = column![
            text("MASTER")
                .size(f32::from(TypeScale::CAPTION))
                .color(colors.text),
            overlays::property_row(
                "Bus",
                components::slider_control(self.gain / 4.0, |value| Message::SetMasterGain(
                    value * 4.0
                )),
                colors,
            ),
            components::progress(self.record_peak, colors),
        ]
        .spacing(f32::from(Spacing::XXS));

        let content = column![rows, master]
            .spacing(f32::from(Spacing::SM))
            .padding(f32::from(Spacing::SM))
            .width(Length::Fill)
            .height(Length::Fill);

        scrollable(content).into()
    }

    fn transport_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let play_glyph = if self.playing { "⏸" } else { "▶" };
        let recording = self.recording.is_some();
        let record_label = if recording {
            if self.punch_in {
                "Stop & punch"
            } else {
                "Stop & import"
            }
        } else if self.punch_in {
            "Punch in"
        } else {
            "Record"
        };
        let record_kind = if recording {
            ButtonKind::Danger
        } else {
            ButtonKind::Primary
        };
        let elapsed = self
            .record_started
            .map_or(0.0, |started| started.elapsed().as_secs_f64());
        column![
            row![
                components::icon_button("⏮", colors, Some(Message::Stop)),
                components::icon_button(play_glyph, colors, Some(Message::PlayPause)),
                components::icon_button("⏹", colors, Some(Message::Stop)),
                text(format!(
                    "{} / {}",
                    format_time(self.position_secs),
                    format_time(self.media_duration)
                ))
                .size(f32::from(TypeScale::BODY_STRONG))
                .font(fonts::medium())
                .color(colors.text),
                Space::new().width(Length::Fill),
                components::muted(format!("{:.0}% gain", self.gain * 100.0), colors),
            ]
            .spacing(f32::from(Spacing::XS))
            .align_y(Alignment::Center),
            components::progress(
                (self.position_secs / self.media_duration.max(1.0)).clamp(0.0, 1.0),
                colors
            ),
            row![
                components::dropdown(
                    self.input_devices.clone(),
                    self.record_device.clone(),
                    Message::SelectInput
                ),
                components::toggle(
                    "Monitor",
                    self.record_monitor,
                    colors,
                    Message::ToggleMonitor
                ),
                components::toggle("Punch", self.punch_in, colors, Message::TogglePunchIn),
                components::button(
                    record_label,
                    record_kind,
                    colors,
                    Some(Message::ToggleRecord)
                ),
            ]
            .spacing(f32::from(Spacing::XS))
            .align_y(Alignment::Center),
            row![
                components::muted(format!("peak {:>5.1}%", self.record_peak * 100.0), colors),
                components::progress(self.record_peak, colors),
                components::muted(format!("rms {:>5.1}%", self.record_rms * 100.0), colors),
                components::progress(self.record_rms, colors),
                components::muted(
                    if recording {
                        format!("● {elapsed:.1}s")
                    } else {
                        "ready".into()
                    },
                    colors
                ),
            ]
            .spacing(f32::from(Spacing::XS))
            .align_y(Alignment::Center),
        ]
        .spacing(f32::from(Spacing::MD))
        .into()
    }

    fn timeline_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let x = self.waveform.visible();
        let waveform = container(self.waveform.view())
            .width(Length::Fill)
            .height(Length::Fixed(120.0))
            .style(move |_theme| colors.card_style());

        let mut tracks = column![].spacing(f32::from(Spacing::XXS));
        for (index, name) in ["Voice", "Music", "SFX"].iter().enumerate() {
            let selected = index == self.selected_track;
            let row = components::button(
                format!("{index:02}  {name}"),
                if selected {
                    ButtonKind::Secondary
                } else {
                    ButtonKind::Ghost
                },
                colors,
                Some(Message::SelectTrack(index)),
            );
            tracks = tracks.push(row);
        }

        column![
            row![
                components::icon_button("−", colors, Some(Message::ZoomWaveform(1.25))),
                components::icon_button("+", colors, Some(Message::ZoomWaveform(0.8))),
                components::icon_button("⟲", colors, Some(Message::SeekWaveform(0.0))),
                components::muted(format!("view {:.1}s → {:.1}s", x.0, x.1), colors),
                Space::new().width(Length::Fill),
                components::muted("Scroll over waveform to zoom", colors),
            ]
            .spacing(f32::from(Spacing::XS))
            .align_y(Alignment::Center),
            waveform,
            scrollable(tracks).height(Length::Fill),
        ]
        .spacing(f32::from(Spacing::SM))
        .into()
    }

    fn inspector_view(&self, colors: ThemeColors) -> Element<'_, Message> {
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
                    colors,
                    Some(Message::SelectClip(clip.id)),
                ));
            }
        }
        let undo = self.session.can_undo().then_some(Message::UndoEdit);
        let redo = self.session.can_redo().then_some(Message::RedoEdit);
        scrollable(
            column![
                components::section_title("Single-track edit", colors),
                components::muted(self.session.status(), colors),
                row![
                    components::button("Undo", ButtonKind::Ghost, colors, undo),
                    components::button("Redo", ButtonKind::Ghost, colors, redo),
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
                scrollable(clips).height(Length::Fixed(112.0)),
                components::kicker("Selection (seconds)", colors),
                row![
                    text_input("start", &self.selection_start)
                        .on_input(Message::SelectionStart)
                        .padding(Spacing::XS)
                        .width(Length::Fill),
                    text_input("end", &self.selection_end)
                        .on_input(Message::SelectionEnd)
                        .padding(Spacing::XS)
                        .width(Length::Fill),
                    components::button(
                        "Apply",
                        ButtonKind::Secondary,
                        colors,
                        Some(Message::ApplySelection)
                    ),
                ]
                .spacing(f32::from(Spacing::XS)),
                row![
                    components::button(
                        "Cut",
                        ButtonKind::Secondary,
                        colors,
                        Some(Message::CutSelection)
                    ),
                    components::button(
                        "Copy",
                        ButtonKind::Ghost,
                        colors,
                        Some(Message::CopySelection)
                    ),
                    components::button(
                        "Paste",
                        ButtonKind::Ghost,
                        colors,
                        Some(Message::PasteClipboard)
                    ),
                    components::button(
                        "Delete",
                        ButtonKind::Danger,
                        colors,
                        Some(Message::DeleteSelection)
                    ),
                ]
                .spacing(f32::from(Spacing::XXS)),
                row![
                    components::button(
                        "Silence",
                        ButtonKind::Secondary,
                        colors,
                        Some(Message::SilenceSelection)
                    ),
                    components::button(
                        "Trim",
                        ButtonKind::Secondary,
                        colors,
                        Some(Message::TrimSelection)
                    ),
                    components::button(
                        "Split",
                        ButtonKind::Secondary,
                        colors,
                        Some(Message::SplitSelection)
                    ),
                    components::button(
                        "Crossfade",
                        ButtonKind::Secondary,
                        colors,
                        Some(Message::CrossfadeSelection(1.0))
                    ),
                    components::button(
                        "Duplicate",
                        ButtonKind::Ghost,
                        colors,
                        Some(Message::DuplicateSelection)
                    ),
                ]
                .spacing(f32::from(Spacing::XXS)),
                row![
                    components::button(
                        "Normalize",
                        ButtonKind::Ghost,
                        colors,
                        Some(Message::NormalizeSelection)
                    ),
                    components::button(
                        "Reverse",
                        ButtonKind::Ghost,
                        colors,
                        Some(Message::ToggleReverse)
                    ),
                    components::button(
                        "Invert",
                        ButtonKind::Ghost,
                        colors,
                        Some(Message::ToggleInvert)
                    ),
                ]
                .spacing(f32::from(Spacing::XXS)),
                row![
                    components::button(
                        "Dip envelope",
                        ButtonKind::Ghost,
                        colors,
                        Some(Message::DipEnvelope)
                    ),
                    components::button(
                        "Flat envelope",
                        ButtonKind::Ghost,
                        colors,
                        Some(Message::ResetEnvelope)
                    ),
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
        )
        .height(Length::Fill)
        .into()
    }

    fn media_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let mut rows = column![].spacing(f32::from(Spacing::XXS));
        for (index, item) in self.media.iter().enumerate() {
            let selected = index == self.selected_media;
            rows = rows.push(components::button(
                format!(
                    "{}  ·  {}  ·  {} ch",
                    item.name,
                    format_time(item.duration_secs),
                    item.channels
                ),
                if selected {
                    ButtonKind::Secondary
                } else {
                    ButtonKind::Ghost
                },
                colors,
                Some(Message::SelectMedia(index)),
            ));
        }
        column![
            row![
                components::button(
                    "Import",
                    ButtonKind::Primary,
                    colors,
                    Some(Message::ImportMedia)
                ),
                components::button(
                    "Rebuild peaks",
                    ButtonKind::Secondary,
                    colors,
                    Some(Message::BuildPeaks)
                ),
            ]
            .spacing(f32::from(Spacing::XS)),
            scrollable(rows).height(Length::Fill),
            components::muted(
                format!(
                    "{} source(s) · 48 kHz · {:.1}s",
                    self.media.len(),
                    self.media_duration
                ),
                colors
            ),
        ]
        .spacing(f32::from(Spacing::SM))
        .into()
    }

    fn voice_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        column![
            components::section_title("Voice Lab", colors),
            components::muted("Local synthesis and cloning stubs", colors),
            components::dropdown(
                self.voices.clone(),
                Some(self.selected_voice.clone()),
                Message::SelectVoice
            ),
            components::button("Synthesize line", ButtonKind::Primary, colors, None),
            overlays::card(
                components::muted(
                    "The editor, transcript import and streaming preview land in Phase 3. This panel proves the glass, focus and dock behaviour.",
                    colors
                ),
                colors
            ),
        ]
        .spacing(f32::from(Spacing::SM))
        .into()
    }

    fn effects_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let mut rows = column![].spacing(f32::from(Spacing::XXS));
        for (index, effect) in self.effects.iter().enumerate() {
            let selected = index == self.selected_effect;
            rows = rows.push(components::button(
                format!(
                    "{}  {}",
                    if effect.enabled { "●" } else { "○" },
                    effect.name
                ),
                if selected {
                    ButtonKind::Secondary
                } else {
                    ButtonKind::Ghost
                },
                colors,
                Some(Message::SelectEffect(index)),
            ));
        }
        column![
            row![
                components::section_title("Effects", colors),
                Space::new().width(Length::Fill),
                components::button("Add", ButtonKind::Secondary, colors, None),
            ]
            .align_y(Alignment::Center),
            scrollable(rows).height(Length::Fill),
            components::muted("Chain state is a Phase 2 stub.", colors),
        ]
        .spacing(f32::from(Spacing::SM))
        .into()
    }

    fn models_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let mut rows = column![].spacing(f32::from(Spacing::SM));
        for model in &self.models {
            rows = rows.push(overlays::card(
                row![
                    column![
                        text(model.name.clone())
                            .size(f32::from(TypeScale::BODY_STRONG))
                            .font(fonts::semibold())
                            .color(colors.text),
                        components::muted(format!("{} · {}", model.purpose, model.size), colors),
                    ]
                    .spacing(f32::from(Spacing::XXS)),
                    Space::new().width(Length::Fill),
                    components::muted(
                        if model.installed {
                            "Installed"
                        } else {
                            "Available"
                        },
                        colors
                    ),
                ]
                .align_y(Alignment::Center),
                colors,
            ));
        }
        column![
            row![
                components::section_title("Models", colors),
                Space::new().width(Length::Fill),
                components::muted("Local only", colors),
            ],
            scrollable(rows).height(Length::Fill),
            components::muted(self.glass.detail.clone(), colors),
            row![
                components::toggle(
                    "Reduce motion",
                    self.reduce_motion,
                    colors,
                    Message::ToggleReduceMotion
                ),
                components::button(
                    "Cycle theme",
                    ButtonKind::Secondary,
                    colors,
                    Some(Message::ToggleTheme)
                ),
            ]
            .spacing(f32::from(Spacing::MD))
            .align_y(Alignment::Center),
            components::muted(
                self.last_save_error
                    .clone()
                    .unwrap_or_else(|| "Workspace saves automatically".into()),
                colors
            ),
        ]
        .spacing(f32::from(Spacing::SM))
        .into()
    }

    fn export_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let mut rows = column![].spacing(f32::from(Spacing::SM));
        if self.jobs.jobs().is_empty() {
            rows = rows.push(components::muted("No export jobs yet.", colors));
        }
        for job in self.jobs.jobs() {
            let status = match job.status {
                JobStatus::Running => format!("{:.0}%", job.progress * 100.0),
                JobStatus::Completed => "Done".into(),
                JobStatus::Cancelled => "Cancelled".into(),
                JobStatus::Failed => "Failed".into(),
            };
            rows = rows.push(overlays::card(
                column![
                    row![
                        text(job.name.clone())
                            .size(f32::from(TypeScale::BODY))
                            .font(fonts::medium())
                            .color(colors.text),
                        Space::new().width(Length::Fill),
                        components::muted(status, colors),
                        if job.status.is_running() {
                            components::button(
                                "Cancel",
                                ButtonKind::Ghost,
                                colors,
                                Some(Message::CancelJob(job.id)),
                            )
                        } else {
                            Space::new().width(Length::Fixed(0.0)).into()
                        },
                    ]
                    .align_y(Alignment::Center)
                    .spacing(f32::from(Spacing::XS)),
                    components::progress(job.progress, colors),
                    components::muted(job.detail.clone(), colors),
                ]
                .spacing(f32::from(Spacing::XS)),
                colors,
            ));
        }
        column![
            row![
                components::section_title("Export Queue", colors),
                components::muted(
                    if self.jobs.running_count() > 0 {
                        format!("{} running", self.jobs.running_count())
                    } else {
                        "idle".into()
                    },
                    colors
                ),
                Space::new().width(Length::Fill),
                components::button(
                    "Export mix",
                    ButtonKind::Primary,
                    colors,
                    Some(Message::ExportMix)
                ),
                components::button(
                    "Clear done",
                    ButtonKind::Ghost,
                    colors,
                    Some(Message::ClearJobs)
                ),
            ]
            .spacing(f32::from(Spacing::XS))
            .align_y(Alignment::Center),
            scrollable(rows).height(Length::Fill),
            components::muted(
                self.session.last_export().unwrap_or_else(|| {
                    if self.jobs.has_running() {
                        "A worker is running; the UI stays responsive."
                    } else {
                        "24-bit WAV · background render"
                    }
                }),
                colors
            ),
        ]
        .spacing(f32::from(Spacing::SM))
        .into()
    }
}
