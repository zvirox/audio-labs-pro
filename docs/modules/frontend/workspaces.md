# `frontend/src/workspaces`

## Purpose

Provides the 7 ported studio workspaces for Audio Labs Pro: Settings, Models, Voices, Generate, Effects, Captures, and Stories. Replaces legacy iced views with high-performance React / Tailwind / TypeScript components built on the design system kit primitives. Enforces mandatory affirmative legal consent gates for voice cloning.

## Public API

- `SettingsWorkspace`: Audio hardware enumeration (ALSA/PipeWire), Appearance & KWin glass report, AI control & MCP security tiers, storage paths, delivery export presets.
- `ModelsWorkspace`: Neural model catalogue with SPDX licenses, SHA-256 verification telemetry, offline enforcement policy, and acceleration status.
- `VoicesWorkspace`: Voice profile library, acoustic faders (speed, pitch, energy, emotion), test phrase synthesis, and cloning trigger.
- `CloningConsentModal`: Mandatory affirmative legal consent gate dialog requiring non-empty speaker name, attester, reference sample, and consent statement. Strictly human-gated.
- `GenerateWorkspace`: Speech prompt composer, voice dropdown, synthesis speed/pitch controls, take queue, and take history feed.
- `EffectsWorkspace`: 6-stage vocal cleanup chain (hum, clicks, 4-band parametric EQ, de-esser, compressor, limiter) with per-stage bypass toggles, signal path metrics, and sandboxed CLAP / LV2 hosted plugin browser with rescan action.
- `CapturesWorkspace`: Take recording deck with input meter, clipping detection, punch-in/monitor toggles, takes list, and interactive word-level transcript editor with click-to-seek and segment cutting.
- `StoriesWorkspace`: Chapter script editor, multi-character cast assignment, line status tracking, batch synthesis, and timeline stitching.

## Data flow

1. Workspaces invoke asynchronous getters in `frontend/src/ipc/client` (`getAudioDevices`, `getModelCatalogue`, `getVoiceProfiles`, `getEffectsChain`, `getDiscoveredPlugins`, `getCaptures`, `getTranscript`, `getStories`).
2. When running inside Tauri (`isTauriEnvironment()`), requests dispatch via `invoke` to corresponding Rust commands in `alp-desktop` and domain handlers in `alp-session`.
3. In browser or mock environments, client fallbacks return deterministic mock datasets matching production schemas.
4. Voice cloning requires affirmative checkbox submission and validation through `CloningConsentModal` before emitting profile creation commands to `alp-session`.
5. Word-level seek clicks in `CapturesWorkspace` dispatch timestamp seek requests directly to the audio transport hook.

## Allowed dependencies

- `frontend/src/kit/*` design system primitives
- `frontend/src/ipc/types` and `frontend/src/ipc/client`
- Lucide React icons
- React standard hooks (`useState`, `useEffect`, `useCallback`)

## Threading / real-time notes

- All workspace rendering executes on the webview UI thread.
- Heavy DSP, model inference, and audio playback execute off-thread in Rust background worker threads and real-time audio threads (`alp-engine`, `alp-voice`).
- State updates are debounced or throttled where appropriate to maintain 60 FPS rendering under active metering and transport updates.

## Error handling

- Async IPC calls catch errors and display actionable inline notices or fallback states.
- Mandatory legal consent gates validate non-empty inputs and string length thresholds prior to enabling submission.
- Audio device and model discovery tolerate empty or missing hardware gracefully.

## Testing

- Verified via `(cd frontend && npm run build)` TypeScript compilation and Vite bundling.
- Verified keyboard accessibility, focus rings, and high contrast styling.
- Verified interactive consent modal validation boundaries and take playback seek callbacks.

## Known limits

- Waveform clip envelope editing and multi-track arranger canvas remain floating panel workflows scheduled for Phase M6 timeline editor.
