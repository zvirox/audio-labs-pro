# Phase 17 Results — VST3 Plugin Host & Batch Macros

Date: 2026-11-26

## Deliverables

- `alp-engine::macro_chain`: offline directory batch processor with progress/cancel callback.
- `alp-api::macro_ops`: `dsp_macro_execute` job wrapper with sandbox validation.
- `alp-mcp`: 69th tool `dsp_macro_execute` with schema, annotations, dispatch and docs.
- `alp-plugin::vst3`: `.vst3` bundle discovery in `/usr/lib/vst3`, `/usr/local/lib/vst3`, `$HOME/.vst3`, `VST3_PATH`; `PluginFormat::Vst3`.
- `alp-desktop::plugin_window`: safe `x11rb` reparent/map bridge with registry and Tauri commands.
- `alp-session::macro_bridge`: desktop macro invocation; Effects workspace batch card and VST3 plugin badge.

## Verification

`cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo fmt --all -- --check`, all four guards, `check:linear`, frontend build/test passed. MCP tool count asserted at 69.

## Honest limits

- VST3 **instantiation is intentionally unsupported**: no C++ COM ABI host is implemented. Descriptors are discovered and `ActivePlugin::instantiate` returns a clear error instead of a fake host.
- Consequently no real VST3 editor uses the X11 bridge yet; it is validated by registry/state tests, not a live plugin window.
- Macro `pattern` is a substring filter; output is 24-bit WAV; cancellation is between files.

## Acceptance harness

`bash scripts/phase17-demo.sh`
