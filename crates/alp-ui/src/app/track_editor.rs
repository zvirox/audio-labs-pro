//! Story track editor dock: a widget-built timeline with gold clip blocks.
//!
//! iced has no absolute layout, so the lane is assembled from weighted fill
//! portions: one block per clip plus spacer blocks for the gaps between them.
//! Portions approximate the real timeline but stay proportional, and every
//! block carries the same selection message the inspector uses.

use alp_ui_kit::components;
use alp_ui_kit::tokens::{Spacing, TypeScale};
use alp_ui_kit::{ButtonKind, ThemeColors, fonts};
use iced::widget::{Space, column, container, mouse_area, row, text};
use iced::{Alignment, Background, Border, Element, Length};

use super::{App, Message, format_time};

const LANE_HEIGHT: f32 = 44.0;
const GUTTER_WIDTH: f32 = 44.0;

impl App {
    /// The docked track editor used by the Stories workspace.
    pub(super) fn track_editor_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let total_secs = self.session.duration_secs().max(1.0);
        let play_glyph = if self.playing { "❚❚" } else { "▶" };

        let toolbar = row![
            components::button(
                play_glyph,
                ButtonKind::Primary,
                colors,
                Some(Message::PlayPause)
            ),
            components::glyph_button("⏹", 13.0, ButtonKind::Ghost, colors, Some(Message::Stop)),
            text(format!(
                "{} / {}",
                format_time(self.position_secs),
                format_time(total_secs as f32)
            ))
            .size(f32::from(TypeScale::CAPTION))
            .font(fonts::mono())
            .color(colors.muted),
            Space::new().width(Length::Fill),
            components::with_tooltip(
                components::glyph_button(
                    "✂",
                    14.0,
                    ButtonKind::Ghost,
                    colors,
                    Some(Message::SplitSelection),
                ),
                "Split at the selection".to_string(),
                colors,
            ),
            components::with_tooltip(
                components::glyph_button(
                    "⧉",
                    14.0,
                    ButtonKind::Ghost,
                    colors,
                    Some(Message::CopySelection),
                ),
                "Copy the selection".to_string(),
                colors,
            ),
            components::with_tooltip(
                components::glyph_button(
                    "⌫",
                    14.0,
                    ButtonKind::Ghost,
                    colors,
                    Some(Message::DeleteSelection),
                ),
                "Delete the selection".to_string(),
                colors,
            ),
            Space::new().width(Length::Fixed(f32::from(Spacing::MD))),
            components::with_tooltip(
                components::button(
                    if self.stories_tools_open {
                        "Hide mixer"
                    } else {
                        "Mixer"
                    },
                    if self.stories_tools_open {
                        ButtonKind::Secondary
                    } else {
                        ButtonKind::Ghost
                    },
                    colors,
                    Some(Message::ToggleStoriesTools),
                ),
                "Mixer and inspector".to_string(),
                colors,
            ),
        ]
        .spacing(f32::from(Spacing::XS))
        .align_y(Alignment::Center);

        let mut lanes = column![].spacing(1.0);
        for (index, track) in self.session.tracks().iter().enumerate() {
            lanes = lanes.push(self.track_lane(index, &track.name, colors, total_secs as f32));
        }

        container(
            column![
                container(toolbar)
                    .width(Length::Fill)
                    .padding([Spacing::SM, Spacing::LG]),
                self.ruler(colors, total_secs as f32),
                lanes,
            ]
            .spacing(0.0),
        )
        .width(Length::Fill)
        .height(Length::Fixed(238.0))
        .style(move |_theme| container::Style {
            background: Some(Background::Color(colors.background)),
            border: Border {
                width: 1.0,
                color: colors.border,
                radius: 0.0.into(),
            },
            shadow: iced::Shadow::default(),
            text_color: Some(colors.text),
            snap: true,
        })
        .into()
    }

    fn ruler(&self, colors: ThemeColors, total_secs: f32) -> Element<'_, Message> {
        let mut ticks = row![].width(Length::Fill);
        for index in 0..6 {
            let secs = total_secs * index as f32 / 5.0;
            ticks = ticks.push(
                container(
                    text(format_time(secs))
                        .size(10.0)
                        .font(fonts::mono())
                        .color(colors.faint),
                )
                .width(Length::Fill)
                .align_x(if index == 5 {
                    Alignment::End
                } else {
                    Alignment::Start
                }),
            );
        }
        container(
            row![Space::new().width(Length::Fixed(GUTTER_WIDTH)), ticks,]
                .height(Length::Fixed(22.0))
                .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding([Spacing::XS, Spacing::MD])
        .style(move |_theme| container::Style {
            border: Border {
                width: 1.0,
                color: colors.border,
                radius: 0.0.into(),
            },
            ..container::Style::default()
        })
        .into()
    }

    fn track_lane(
        &self,
        index: usize,
        name: &str,
        colors: ThemeColors,
        total_secs: f32,
    ) -> Element<'_, Message> {
        let mut blocks = row![]
            .width(Length::Fill)
            .height(Length::Fixed(LANE_HEIGHT));
        let mut cursor = 0.0_f64;
        if let Some(track) = self.session.tracks().get(index) {
            for clip in &track.clips {
                let start =
                    super::frames_to_secs(clip.timeline_start, self.session.project().sample_rate);
                let end = super::frames_to_secs(
                    self.session.project().clip_timeline_end(clip),
                    self.session.project().sample_rate,
                );
                let gap = (start - cursor).max(0.0);
                if gap > 0.0 {
                    blocks = blocks.push(
                        Space::new()
                            .width(Length::FillPortion(portion(gap as f32, total_secs)))
                            .height(Length::Fixed(LANE_HEIGHT)),
                    );
                }
                let selected = self.session.selected_clip() == Some(clip.id);
                let duration = (end - start).max(0.0) as f32;
                let label = self
                    .session
                    .project()
                    .source(clip.source_id)
                    .map_or_else(|| name.to_owned(), |source| source.name.clone());
                let label = shorten(&label, 18);
                blocks = blocks.push(
                    mouse_area(
                        container(
                            column![
                                text(label)
                                    .size(10.0)
                                    .font(fonts::semibold())
                                    .color(colors.text),
                                components::progress(clip.gain.clamp(0.0, 1.0), colors),
                            ]
                            .spacing(2.0)
                            .padding([3.0, 6.0]),
                        )
                        .width(Length::FillPortion(portion(duration, total_secs)))
                        .height(Length::Fixed(LANE_HEIGHT - 6.0))
                        .style(move |_theme| clip_style(selected, colors)),
                    )
                    .on_press(Message::SelectClip(clip.id)),
                );
                cursor = end;
            }
        }
        let tail = (f64::from(total_secs) - cursor).max(0.0);
        if tail > 0.0 {
            blocks = blocks.push(
                Space::new()
                    .width(Length::FillPortion(portion(tail as f32, total_secs)))
                    .height(Length::Fixed(LANE_HEIGHT)),
            );
        }

        let gutter = mouse_area(
            container(
                text(format!("{:02}", index + 1))
                    .size(11.0)
                    .font(fonts::mono())
                    .color(if index == self.selected_track {
                        colors.accent
                    } else {
                        colors.faint
                    }),
            )
            .width(Length::Fixed(GUTTER_WIDTH))
            .height(Length::Fixed(LANE_HEIGHT))
            .align_x(Alignment::Center)
            .align_y(iced::alignment::Vertical::Center),
        )
        .on_press(Message::SelectTrack(index));

        container(
            row![
                gutter,
                container(blocks).padding([3.0, 0.0]).width(Length::Fill)
            ]
            .height(Length::Fixed(LANE_HEIGHT)),
        )
        .width(Length::Fill)
        .style(move |_theme| container::Style {
            border: Border {
                width: 1.0,
                color: colors.grid,
                radius: 0.0.into(),
            },
            ..container::Style::default()
        })
        .into()
    }
}

fn clip_style(selected: bool, colors: ThemeColors) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(if selected {
            iced::Color {
                a: 0.30,
                ..colors.accent
            }
        } else {
            iced::Color {
                a: 0.18,
                ..colors.accent
            }
        })),
        text_color: Some(colors.text),
        border: Border {
            radius: f32::from(alp_ui_kit::Radius::SM).into(),
            width: 1.0,
            color: if selected {
                colors.accent
            } else {
                iced::Color {
                    a: 0.45,
                    ..colors.accent
                }
            },
        },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

fn portion(secs: f32, total_secs: f32) -> u16 {
    let ratio = (secs / total_secs.max(0.001)).clamp(0.0, 1.0);
    ((ratio * 1_000.0).round() as u16).max(1)
}

fn shorten(text: &str, max: usize) -> String {
    if text.chars().count() > max {
        let short: String = text.chars().take(max).collect();
        format!("{short}…")
    } else {
        text.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::{portion, shorten};

    #[test]
    fn portions_stay_positive_and_proportional() {
        assert_eq!(portion(0.0, 10.0), 1);
        assert_eq!(portion(5.0, 10.0), 500);
        assert_eq!(portion(20.0, 10.0), 1_000);
    }

    #[test]
    fn labels_shorten_on_char_boundaries() {
        assert_eq!(shorten("Notch", 8), "Notch");
        assert_eq!(shorten("Linus Sebastian", 5), "Linus…");
    }
}
