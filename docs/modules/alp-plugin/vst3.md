# `alp-plugin` / `vst3`

## Purpose

VST3 bundle discovery, C++ COM ABI instantiation, audio/parameter processing, and X11 editor hosting for Linux. Scans standard `.vst3` bundle locations, parses `IPluginFactory`/`IPluginFactory2` metadata, and instantiates `IComponent`/`IAudioProcessor`/`IEditController` plugins behind `ActivePlugin`.

## Public API

- `scan_vst3_bundle(path) -> Result<PluginDescriptor, PluginError>`.
- `ActiveVst3Instance` (wrapped cleanly inside `ActivePlugin::Vst3`) implementing parameter enumeration/automation, audio block processing, and `open_editor_x11`/`close_editor_x11`.
- `PluginFormat::Vst3` with display name `VST3` and extension `vst3`.

## Data flow

1. Locate a `.vst3` bundle directory under `/usr/lib/vst3`, `/usr/local/lib/vst3`, `$HOME/.vst3`, or `VST3_PATH`, resolve its Linux `.so`, load `ModuleEntry` / `GetPluginFactory`, and read `PClassInfo`/`PClassInfo2` (`vendor`, `version`, `name`).
2. `ActivePlugin::instantiate` initializes `IComponent` with `Vst3HostApp`, queries `IAudioProcessor` and `IEditController` (either directly or via `getControllerClassId`), connects component/controller points, sets `HostComponentHandler`, configures stereo bus arrangements, activates buses, and calls `setupProcessing` + `setActive(true)` + `setProcessing(true)`.
3. Zero-allocation `HostParameterChanges` / `HostParamValueQueue` delivers normalized parameter automation during `ProcessData` processing calls; `open_editor_x11` creates `"editor"` `IPlugView`, verifies `"X11EmbedWindowID"`, binds `IPlugFrame`, and attaches to the host XID.

## Allowed dependencies

`alp-core` (via plugin types), `serde`, `libloading`, and `vst3` (`0.3.0`, `MIT OR Apache-2.0` pre-generated Steinberg VST3 COM bindings).

## Threading / real-time notes

Discovery, `ModuleEntry`, COM setup, parameter synchronization, and `IPlugView` attachment happen off the audio thread. The real-time `process` path uses preallocated scratch vectors and fixed parameter queues with no heap allocation during callbacks.

## Error handling

Non-directory paths, missing `.so` payloads, missing `GetPluginFactory`, missing `"Audio Module Class"` entries, failed `IComponent`/`IAudioProcessor` setup, or unsupported `"X11EmbedWindowID"` views return descriptive `PluginError` variants.

## Testing

Unit tests in `vst3.rs` verify invalid bundle/binary errors, and integration tests in `tests/vst3_host.rs` compile a real C shared-library `.vst3` implementing VST3 COM vtables and verify scan metadata, instantiation, gain automation, stereo processing, and X11 `IPlugView` embedding.

## Known limits

- Linux `.vst3` bundles (`Contents/<arch>-linux/*.so` or top-level `.so`) and `"X11EmbedWindowID"` editor parenting only; Wayland-native editor views are not yet bridged.
- Multi-bus surround routing and MIDI event lists are normalized onto primary stereo audio + normalized parameters in this implementation.
