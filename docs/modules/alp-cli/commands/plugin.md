# `alp-cli` / `commands/plugin`

## Purpose

Provides command-line interfaces to discover, list, and inspect CLAP and LV2 audio plugins
installed on the host Linux system or bundled with Audio Labs Pro.

## Public API

- `scan(custom_path: Option<&Path>, json: bool) -> Result<()>`: Scan standard paths or a custom path for CLAP/LV2 plugins and update the discovery cache.
- `list(json: bool) -> Result<()>`: List all discovered plugins with their formats, categories, and paths.
- `inspect(plugin_id: &str, json: bool) -> Result<()>`: Instantiate and inspect a plugin's exposed parameters, value ranges, and defaults.

## Data flow

1. `scan` invokes `alp_plugin::PluginScanner` over standard directories (`~/.clap`, `/usr/lib/clap`, `~/.lv2`, `/usr/lib/lv2`) or a custom user path, writing descriptors to `$XDG_CACHE_HOME/alp/plugins.json`.
2. `list` queries `alp_plugin::PluginHost` to display cached, installed, and virtual built-in plugins.
3. `inspect` loads the target plugin via C ABI or virtual fallback, reads parameter descriptors, and outputs formatted tables or JSON.

## Allowed dependencies

- `alp-plugin`
- `anyhow`
- `serde_json`
- `std::path::Path`

## Threading / real-time notes

Runs synchronously on the CLI thread during tool invocation. Dynamic library inspection occurs offline before audio playback.

## Error handling

Returns detailed `anyhow::Result` contexts when plugin files fail validation, C ABI symbols are missing, or plugins cannot be located.

## Testing

Verified via acceptance demo `scripts/phase11-demo.sh` and CLI integration checks with real LV2 bundles in `/usr/lib/lv2`.

## Known limits

CLAP GUI extensions (`clap_plugin_gui`) and X11/Wayland plugin windows are inspected headless; GUI embedding is handled in `alp-ui`.
