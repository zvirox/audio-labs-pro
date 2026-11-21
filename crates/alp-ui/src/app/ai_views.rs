//! Compact AI automation controls and the local audit trail.
//!
//! This body is embedded in the Settings workspace card; it deliberately avoids
//! its own titles and outer scrolling so the card can size to its content.

use alp_api::PermissionTier;
use alp_ui_kit::components;
use alp_ui_kit::tokens::{Spacing, TypeScale};
use alp_ui_kit::{ButtonKind, ThemeColors, fonts};
use iced::widget::{Space, column, container, row, text};
use iced::{Alignment, Element, Length};

use super::{App, Message};

impl App {
    /// MCP status, permission tier, kill switch and the AI audit trail.
    pub(super) fn ai_activity_view(&self, colors: ThemeColors) -> Element<'_, Message> {
        let status_badge = if self.ai_enabled {
            components::button(
                "Stop MCP",
                ButtonKind::Danger,
                Some(Message::ToggleAiControl),
            )
        } else {
            components::button(
                "Allow MCP",
                ButtonKind::Secondary,
                Some(Message::ToggleAiControl),
            )
        };

        let header = row![
            column![
                components::kicker(
                    if self.ai_enabled {
                        "MCP attach · active"
                    } else {
                        "MCP attach · standby"
                    },
                    colors
                ),
                components::muted(&self.ai_status, colors),
            ]
            .spacing(f32::from(Spacing::XXS)),
            Space::new().width(Length::Fill),
            status_badge,
        ]
        .align_y(Alignment::Center)
        .spacing(f32::from(Spacing::SM));

        let tier = match self.ai_tier {
            PermissionTier::Observe => 0,
            PermissionTier::Edit => 1,
            PermissionTier::Export => 2,
        };
        let tier_buttons =
            components::segmented(&["Observe", "Edit", "Export"], tier, colors, |index| {
                Message::SetAiPermissionTier(match index {
                    0 => PermissionTier::Observe,
                    1 => PermissionTier::Edit,
                    _ => PermissionTier::Export,
                })
            });

        let tier_hint = match self.ai_tier {
            PermissionTier::Observe => {
                "Observe: attached AI can read the current project and tracks."
            }
            PermissionTier::Edit => {
                "Edit: the full MCP catalog can edit the live project on workers; successful calls share one undo history."
            }
            PermissionTier::Export => {
                "Export: attached AI may also render and export inside the live project's workspace sandbox."
            }
        };

        let transport = if self.ai_enabled {
            let path = self
                .ai_control
                .socket_path
                .as_ref()
                .map_or_else(String::new, |path| path.display().to_string());
            let latency = self.ai_control.mcp.last_commit_ms.map_or_else(
                || "No attached commits yet".into(),
                |latest| {
                    format!(
                        "Request → live commit: {latest} ms latest · {} ms worst of {}. View-model timing, not pixel presentation; preparation and rendering are included in the measurement.",
                        self.ai_control.mcp.max_commit_ms, self.ai_control.mcp.commits
                    )
                },
            );
            column![
                components::muted(
                    "Full shared MCP catalog: ALP_CONTROL_TOKEN=<token> alp-mcp --attach",
                    colors,
                ),
                row![
                    components::chip("Per-session token", colors),
                    components::button(
                        "Copy token",
                        ButtonKind::Secondary,
                        Some(Message::CopyAiToken)
                    ),
                    components::muted("0600 socket · peer UID checked · never persisted", colors),
                ]
                .spacing(f32::from(Spacing::XS))
                .align_y(Alignment::Center),
                components::muted(path, colors),
                components::muted(latency, colors),
            ]
            .spacing(f32::from(Spacing::XXS))
        } else {
            column![components::muted(
                "Enable live attach to create $XDG_RUNTIME_DIR/alp/control.sock. Headless alp-mcp still opens a separate project session.",
                colors,
            )]
        };

        let safety_actions = row![
            components::button("Revoke attach", ButtonKind::Danger, Some(Message::StopAi)),
            components::button(
                "Undo last edit",
                ButtonKind::Secondary,
                Some(Message::UndoAiChanges)
            ),
            Space::new().width(Length::Fill),
            components::chip(format!("{:?}", self.ai_tier), colors),
        ]
        .spacing(f32::from(Spacing::XS))
        .align_y(Alignment::Center);

        let mut log_column = column![].spacing(f32::from(Spacing::XXS));
        if self.ai_activity_log.is_empty() {
            log_column = log_column.push(components::muted("No activity yet.", colors));
        }
        for entry in self.ai_activity_log.iter().rev().take(12) {
            log_column = log_column.push(
                row![
                    text("›")
                        .size(f32::from(TypeScale::CAPTION))
                        .font(fonts::mono())
                        .color(colors.accent),
                    text(entry.clone())
                        .size(f32::from(TypeScale::CAPTION))
                        .font(fonts::regular())
                        .color(colors.text),
                ]
                .spacing(f32::from(Spacing::XS))
                .align_y(Alignment::Center),
            );
        }

        let log_box = container(self.scroll(log_column, colors).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fixed(96.0))
            .padding(Spacing::SM)
            .style(move |_theme| colors.well_style());

        column![
            header,
            tier_buttons,
            components::muted(tier_hint, colors),
            transport,
            safety_actions,
            log_box,
        ]
        .spacing(f32::from(Spacing::SM))
        .into()
    }
}
