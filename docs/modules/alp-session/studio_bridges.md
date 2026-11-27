# `alp-session` / `studio_bridges`

## Purpose

Headless desktop IPC bridge exposing real `alp-engine`, `alp-voice`, `alp-dsp`, and `alp-api` capabilities (audio device enumeration, plugin discovery, neural model verification, hardware probing, voice profile listing, TTS synthesis, lexicon preview, spectral repair, time stretching, loudness analysis, and transcription) to `alp-desktop`.

## Public API

- `list_audio_devices() -> Vec<AudioDeviceDto>`
- `list_model_catalogue() -> Vec<ModelCatalogueItemDto>`, `verify_model_integrity(model_id)`
- `probe_hardware_profile() -> HardwareProfileDto`
- `discover_system_plugins() -> Result<Vec<DiscoveredPluginDto>, String>`
- `list_voice_profiles()`, `synthesize_tts_preview(voice_id, text, speed)`
- `preview_lexicon_markup(rules, text)`
- `desktop_spectral_repair_file`, `desktop_time_stretch_file`, `analyze_file_loudness`, `desktop_transcribe_audio`

## Data flow

Constructs short-lived Export-tier `ApiSession`s, `ModelManager` instances, or pure DSP analyzers off the real-time audio thread and serializes DTOs that align cleanly with `frontend/src/ipc/types.ts`.

## Allowed dependencies

`alp-api`, `alp-core`, `alp-dsp`, `alp-engine`, `alp-media`, `alp-voice`, `serde`. No `tauri`, `iced`, `alp-infer`, or direct `alp-plugin` edge (`check-deps.sh`).

## Threading / real-time notes

All operations execute synchronously on Tauri worker threads off the audio callback.

## Error handling

Invalid paths, missing weights, unattached inference daemons, and codec failures return descriptive `Err(String)` diagnostics.

## Testing

Unit tests verify pronunciation dictionary + markup parsing, non-empty hardware/model telemetry including `demucs-4stem`, and explicit rejection of blank file paths.

## Known limits

- Offline file repair, loudness analysis, and transcription operate on sandboxed filesystem paths accessible to the desktop process.
- Live hardware audio playback uses `TransportController`; device selection displays live OS sinks/inputs.
