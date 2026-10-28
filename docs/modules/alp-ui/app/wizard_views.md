# `alp-ui` / `app` / `wizard_views`

## Purpose

Renders modal overlays for first-run setup (hardware detection, audio devices,
appearance) and the keyboard shortcuts cheat sheet. Both use
`alp-ui-kit::overlays::dialog` with the studio panel style and a footer close
action.

## Public API

- `App::first_run_wizard_view(&self, colors: ThemeColors) -> Element<'_, Message>`
- `App::shortcuts_help_modal_view(&self, colors: ThemeColors) -> Element<'_, Message>`

## Data flow

1. Reads audio devices, probed hardware capability summary, theme mode, and glass status from `App`.
2. Emits configuration messages (`Message::ToggleTheme`, `Message::ToggleReduceMotion`, `Message::FinishFirstRunWizard`, `Message::CloseShortcutsHelp`).
3. Renders three numbered step wells inside `overlays::dialog`; the shortcuts
   sheet groups commands by category in a 340 px scroller.

## Allowed dependencies

- `iced::widget::{column, row, scrollable, text}`
- `iced::{Alignment, Element, Length}`
- `alp_ui_kit::{ButtonKind, ThemeColors, components, fonts, overlays, tokens}`
- `alp_ui_kit::tokens::{Radius, Spacing, TypeScale}`
- `crate::app::{App, Message}`
- `crate::glass::GlassMode`

## Threading / real-time notes

View methods are pure rendering functions; they run exclusively on the main UI thread during frame generation and perform no blocking operations.

## Error handling

Provides clear fallback messaging when audio devices or hardware accelerators are not detected.

## Testing

Covered by iced widget compilation, workspace test harness, and `scripts/check-ui.sh`.

## Known limits

- Hardware tier detection displays CPU cores, total RAM and the recommended tier.
- The shortcuts sheet lists the documented bindings but cannot rebind keys.
- Both modals are opened by `Message` or the `ALP_START_OVERLAY` smoke hook; the
  wizard also opens by itself when the workspace file has no completed first run.
