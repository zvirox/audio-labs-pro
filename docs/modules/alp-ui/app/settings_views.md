# `alp-ui` / `app/settings_views`

## Purpose

Renders the Models and Settings workspaces: the model catalogue with license and
disk status, offline policy, appearance and motion, AI control, storage paths,
delivery presets, the export queue, workspace panels and session actions.

## Public API

- `App::models_workspace(&self, colors: ThemeColors) -> Element<'_, Message>`
- `App::settings_workspace(&self, colors: ThemeColors) -> Element<'_, Message>`

## Data flow

1. Reads `model_manager`, `models_offline`, `profiles`, `theme_mode`,
   `reduce_motion`, `ai_*`, data paths, jobs and `EditSession` state.
2. Emits `AcceptModelLicense`, `RemoveModel`, `ToggleOfflineModels`,
   `ToggleTheme`, `ToggleReduceMotion`, `ToggleAiControl`, tier changes,
   `ExportPreset`, `SaveLayout`, `ResetLayout`, `OpenFirstRunWizard`,
   `OpenShortcutsHelp`, `TogglePanel` and export-queue messages.
3. Views are pure; licence, removal and policy changes run in `studio.rs`.

## Allowed dependencies

- `alp-voice` model types, `alp-media::ExportPreset`, `alp-api::PermissionTier`
- `alp-ui-kit` components/tokens, `iced` widget primitives
- `super::ai_views`, `super::views::export_view`, `crate::panels`

## Threading / real-time notes

No audio work. Export jobs render on their own worker and are polled by the
frame subscription; this view only lists them.

## Error handling

License and removal failures are toasts. Paths that are unavailable (for example
a missing profile store) render as `unavailable` rather than failing the view.

## Testing

Compile-checked and smoke-run; model status and licence gates are covered by
`alp-voice` tests.

## Known limits

- There is no downloader: accepting a licence records consent, and files must be
  placed in the model folder by hand.
- Offline mode defaults to on and only governs policy; no network code exists.
