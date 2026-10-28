# `alp-ui` / `app` / `wizard_views`

## Purpose

Renders modal overlays for first-run initial setup calibration (hardware detection, audio devices, appearance) and the interactive keyboard shortcuts cheat sheet.

## Public API

- `App::first_run_wizard_view(&self, colors: ThemeColors) -> Element<'_, Message>`
- `App::shortcuts_help_modal_view(&self, colors: ThemeColors) -> Element<'_, Message>`

## Data flow

1. Reads audio devices, probed hardware capability summary, theme mode, and glass status from `App`.
2. Emits configuration messages (`Message::ToggleTheme`, `Message::ToggleReduceMotion`, `Message::FinishFirstRunWizard`, `Message::CloseShortcutsHelp`).
3. Renders modal overlay layers using `alp_ui_kit::overlays::modal`.

## Allowed dependencies

- `iced::widget::{column, row, scrollable, text}`
- `iced::{Alignment, Element, Length}`
- `alp_ui_kit::{ButtonKind, ThemeColors, components, fonts, overlays, tokens}`
- `crate::app::{App, Message}`
- `crate::glass::GlassMode`

## Threading / real-time notes

View methods are pure rendering functions; they run exclusively on the main UI thread during frame generation and perform no blocking operations.

## Error handling

Provides clear fallback messaging when audio devices or hardware accelerators are not detected.

## Testing

Covered by iced widget compilation, workspace test harness, and `scripts/check-ui.sh`.

## Known limits

- Hardware tier detection displays CPU cores, total RAM, and GPU render node presence.
