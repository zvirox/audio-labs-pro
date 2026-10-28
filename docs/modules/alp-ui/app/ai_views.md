# `alp-ui` / `app/ai_views`

## Purpose

Renders the AI Control body embedded in the Settings workspace card: the MCP
attach status and kill switch, permission tier segmented control, the
Observe/Edit/Export explanation, one-click rollback and the audit trail.

## Public API

- `App::ai_activity_view(&self, colors: ThemeColors) -> Element<'_, Message>`

## Data flow

Reads `self.ai_enabled`, `self.ai_tier`, `self.ai_status` and
`self.ai_activity_log`. Emits `ToggleAiControl`, `SetAiPermissionTier`,
`StopAi` and `UndoAiChanges`, which are handled by `App::update`. The body has
no card title or outer scroll of its own; the Settings card supplies both.

## Allowed dependencies

- `alp-api`: `PermissionTier`
- `alp-ui-kit`: tokens, fonts, theme colors and components
- `iced`: `column`, `row`, `container`, `text`, `Space` and the themed scroller

## Threading / real-time notes

View generation runs synchronously on the UI render loop. State is stored in
memory and never performs blocking socket or file I/O on the render thread.

## Error handling

Status messages and kill-switch invocations are rendered with semantic button
variants; actions surface feedback through toasts and the audit log. An empty
log shows "No activity yet." rather than an empty box.

## Testing

Covered by workspace UI validation guards (`check-ui.sh`) ensuring compliance
with design-system buttons, and integration tests in the UI crate.

## Known limits

- The control socket client authentication list is currently local-only to the
  user's `$XDG_RUNTIME_DIR`.
- The audit log shows the last 12 entries, newest first, in a fixed 96 px well.
- Permission tiers are enforced by handlers, not by the socket transport.
