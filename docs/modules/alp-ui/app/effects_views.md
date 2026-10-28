# `alp-ui` / `app/effects_views`

## Purpose

Renders the Effects workspace: the voice-cleanup chain with one switch per
stage, the fixed signal path with mastering target metrics, and the hosted
CLAP/LV2 plugin card with rescan and insert actions.

## Public API

- `App::effects_workspace(&self, colors: ThemeColors) -> Element<'_, Message>`

## Data flow

1. Reads `effects`, `selected_effect`, `discovered_plugins` and the master
   defaults from `App`.
2. Emits `SelectEffect`, `ToggleEffect`, `ApplyVoiceCleanupPreset`,
   `InsertPlugin` and `RescanPlugins` through the embedded `effects_view` and
   `plugin_list_view`.
3. Chain state is a UI model today; DSP execution lives in `alp-dsp` and is
   reached by the CLI and export paths.

## Allowed dependencies

- `alp-ui-kit` components/tokens, `iced` widget primitives
- `super::{App, Message}`, `super::workspace_layout::{card, card_auto, header,
  metric_badge}`

## Threading / real-time notes

Pure view. Plugin discovery is triggered by a message and runs in the handler,
not during rendering.

## Error handling

An empty plugin list shows a zero count and a rescan action. The card copy states
that cleanup does not yet mutate the mix. Stage rows remain selectable while
bypassed, so parameters stay inspectable.

## Testing

Compile-checked with the shell; plugin discovery is covered by `alp-plugin`
tests and the CLI.

## Known limits

- The iced shell cannot apply the chain to the live engine: `alp-ui` may not
  depend on `alp-dsp`, and `alp-engine` ignores `EffectRef` today.
- Every plugin is listed, not filtered by format or vendor.
- The plugin list is capped at eight visible rows in the scroll area.
