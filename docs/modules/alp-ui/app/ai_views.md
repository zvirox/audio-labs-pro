# `alp-ui` / `app/ai_views`

## Purpose

Render views for the AI Activity & Control panel, including automation master switch,
permission tier configuration (Observe, Edit, Export), emergency kill switch, one-click
session rollback, and live JSON-RPC tool audit feed.

## Public API

- `App::ai_activity_view`: Builds the panel element hierarchy displaying AI status, controls, and activity trail.

## Data flow

Reads `self.ai_enabled`, `self.ai_tier`, `self.ai_status`, and `self.ai_activity_log`.
Emits shell messages (`ToggleAiControl`, `SetAiPermissionTier`, `StopAi`, `UndoAiChanges`)
which are dispatched to `App::update`.

## Allowed dependencies

- `alp-api`: `PermissionTier`
- `alp-ui-kit`: tokens, fonts, theme colors, and UI components
- `iced`: layout widgets (`column`, `row`, `container`, `scrollable`, `text`, `Space`)

## Threading / real-time notes

View generation runs synchronously on the UI render loop. State is stored in memory and
never performs blocking socket or file I/O on the render thread.

## Error handling

Status messages and kill-switch invocations are rendered with clear visual styling and
surface feedback via toasts.

## Testing

Covered by workspace UI validation guards (`check-ui.sh`) ensuring compliance with design system
button widgets, and integration tests in the UI crate.

## Known limits

- Control socket client authentication list is currently local-only to the user's `$XDG_RUNTIME_DIR`.
