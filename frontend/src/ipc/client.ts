/**
 * Audio Labs Pro - Tauri IPC Client
 *
 * Provides typed invoke wrappers and event listeners.
 * Seamlessly provides development mock fallbacks when previewed outside Tauri runtime.
 */

import { invoke, Channel } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  ProjectSummary,
  WorkspaceState,
  GlassReport,
  SessionRevisionPayload,
  TransportState,
  MeterFrame,
  WaveformPeaksDto,
  AudioDevice,
  ModelItem,
  VoiceProfile,
  ConsentRecord,
  EffectStage,
  DiscoveredPlugin,
  CaptureTake,
  TranscriptWord,
  StoryScript,
} from './types';

export const isTauriEnvironment = (): boolean => {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
};

// In-memory demo state for browser mock mode
let mockRevision = 1;
let mockDirty = false;
let mockWorkspace: WorkspaceState = {
  version: 1,
  theme: 'dark',
  reduce_motion: false,
  glass: 'compositor_blur',
  panels: [
    {
      kind: 'timeline',
      rect: { x: 72, y: 380, width: 880, height: 320 },
      z: 1,
      collapsed: false,
      docked: null,
      open: true,
    },
    {
      kind: 'transport',
      rect: { x: 400, y: 64, width: 440, height: 120 },
      z: 2,
      collapsed: false,
      docked: null,
      open: true,
    },
    {
      kind: 'mixer',
      rect: { x: 72, y: 64, width: 310, height: 300 },
      z: 3,
      collapsed: false,
      docked: null,
      open: true,
    },
    {
      kind: 'inspector',
      rect: { x: 860, y: 64, width: 320, height: 420 },
      z: 4,
      collapsed: false,
      docked: 'right',
      open: true,
    },
    {
      kind: 'voice_lab',
      rect: { x: 260, y: 120, width: 540, height: 380 },
      z: 5,
      collapsed: false,
      docked: null,
      open: false,
    },
  ],
  first_run_completed: true,
};

export async function getProjectSummary(): Promise<ProjectSummary> {
  if (isTauriEnvironment()) {
    return await invoke<ProjectSummary>('get_project_summary');
  }

  return {
    id: 'prj_demo_voice_01',
    name: 'Voice Studio Demo (Browser Mock)',
    revision: mockRevision,
    sample_rate: 48000,
    duration_secs: 20.0,
    track_count: 2,
    clip_count: 4,
    undo_steps: 0,
    redo_steps: 0,
    is_dirty: mockDirty,
  };
}

export async function getWorkspaceState(): Promise<WorkspaceState> {
  if (isTauriEnvironment()) {
    return await invoke<WorkspaceState>('get_workspace_state');
  }
  return JSON.parse(JSON.stringify(mockWorkspace));
}

export async function saveWorkspaceState(workspace: WorkspaceState): Promise<void> {
  if (isTauriEnvironment()) {
    await invoke('save_workspace_state', { workspace });
    return;
  }
  mockWorkspace = JSON.parse(JSON.stringify(workspace));
}

export async function resetWorkspaceLayout(): Promise<WorkspaceState> {
  if (isTauriEnvironment()) {
    return await invoke<WorkspaceState>('reset_workspace_layout');
  }
  mockWorkspace.panels = [
    {
      kind: 'timeline',
      rect: { x: 72, y: 380, width: 880, height: 320 },
      z: 1,
      collapsed: false,
      docked: null,
      open: true,
    },
    {
      kind: 'transport',
      rect: { x: 400, y: 64, width: 440, height: 120 },
      z: 2,
      collapsed: false,
      docked: null,
      open: true,
    },
    {
      kind: 'mixer',
      rect: { x: 72, y: 64, width: 310, height: 300 },
      z: 3,
      collapsed: false,
      docked: null,
      open: true,
    },
    {
      kind: 'inspector',
      rect: { x: 860, y: 64, width: 320, height: 420 },
      z: 4,
      collapsed: false,
      docked: 'right',
      open: true,
    },
  ];
  return JSON.parse(JSON.stringify(mockWorkspace));
}

export async function getGlassReport(): Promise<GlassReport> {
  if (isTauriEnvironment()) {
    return await invoke<GlassReport>('get_glass_report');
  }
  return {
    session: 'x11',
    desktop: 'KDE',
    handle: 'x11/x11',
    mode: 'compositor_blur',
    blur_available: true,
    detail: 'Mock KWin blur active via _KDE_NET_WM_BLUR_BEHIND_REGION.',
  };
}

export async function triggerDemoEdit(action: string): Promise<SessionRevisionPayload> {
  if (isTauriEnvironment()) {
    return await invoke<SessionRevisionPayload>('trigger_demo_edit', { action });
  }

  mockRevision += 1;
  mockDirty = true;
  const payload: SessionRevisionPayload = {
    revision: mockRevision,
    is_dirty: mockDirty,
    reason: `Mock Edit: ${action}`,
  };

  window.dispatchEvent(new CustomEvent('session_revision_changed', { detail: payload }));
  return payload;
}

export async function listenSessionRevision(
  handler: (payload: SessionRevisionPayload) => void
): Promise<UnlistenFn> {
  if (isTauriEnvironment()) {
    return await listen<SessionRevisionPayload>('session_revision_changed', (event) => {
      handler(event.payload);
    });
  }

  const listener = (event: Event) => {
    const custom = event as CustomEvent<SessionRevisionPayload>;
    handler(custom.detail);
  };
  window.addEventListener('session_revision_changed', listener);
  return () => {
    window.removeEventListener('session_revision_changed', listener);
  };
}

// Mock transport state for browser testing
let mockTransport: TransportState = {
  playing: false,
  position_secs: 0.0,
  position_frames: 0,
  duration_secs: 20.0,
  sample_rate: 48000,
  loop_enabled: false,
  loop_start_secs: 0.0,
  loop_end_secs: 20.0,
  volume: 1.0,
  master_peak_l: 0.0,
  master_peak_r: 0.0,
  master_rms_l: 0.0,
  master_rms_r: 0.0,
  anchor_timestamp_ms: 0.0,
};

export async function playTransport(): Promise<TransportState> {
  if (isTauriEnvironment()) {
    return await invoke<TransportState>('play_transport');
  }
  mockTransport.playing = true;
  mockTransport.anchor_timestamp_ms = performance.now();
  return { ...mockTransport };
}

export async function pauseTransport(): Promise<TransportState> {
  if (isTauriEnvironment()) {
    return await invoke<TransportState>('pause_transport');
  }
  mockTransport.playing = false;
  return { ...mockTransport };
}

export async function stopTransport(): Promise<TransportState> {
  if (isTauriEnvironment()) {
    return await invoke<TransportState>('stop_transport');
  }
  mockTransport.playing = false;
  mockTransport.position_secs = mockTransport.loop_enabled ? mockTransport.loop_start_secs : 0.0;
  return { ...mockTransport };
}

export async function seekTransport(positionSecs: number): Promise<TransportState> {
  if (isTauriEnvironment()) {
    return await invoke<TransportState>('seek_transport', { positionSecs });
  }
  mockTransport.position_secs = Math.max(0, Math.min(positionSecs, mockTransport.duration_secs));
  mockTransport.anchor_timestamp_ms = performance.now();
  return { ...mockTransport };
}

export async function setTransportVolume(volume: number): Promise<TransportState> {
  if (isTauriEnvironment()) {
    return await invoke<TransportState>('set_transport_volume', { volume });
  }
  mockTransport.volume = Math.max(0, Math.min(volume, 1.5));
  return { ...mockTransport };
}

export async function setTransportLoop(
  enabled: boolean,
  startSecs: number,
  endSecs: number
): Promise<TransportState> {
  if (isTauriEnvironment()) {
    return await invoke<TransportState>('set_transport_loop', {
      enabled,
      startSecs,
      endSecs,
    });
  }
  mockTransport.loop_enabled = enabled;
  mockTransport.loop_start_secs = startSecs;
  mockTransport.loop_end_secs = endSecs;
  return { ...mockTransport };
}

export async function getTransportState(): Promise<TransportState> {
  if (isTauriEnvironment()) {
    return await invoke<TransportState>('get_transport_state');
  }
  return { ...mockTransport };
}

export async function getWaveformPeaks(): Promise<WaveformPeaksDto> {
  if (isTauriEnvironment()) {
    return await invoke<WaveformPeaksDto>('get_waveform_peaks');
  }

  // Generate 20-second synthetic demo peaks for browser preview
  const sampleRate = 48000;
  const durationSecs = 20.0;
  const totalFrames = sampleRate * durationSecs;
  const bucketSamples = 256;
  const count = Math.floor(totalFrames / bucketSamples);
  const pairs: [number, number][] = [];

  for (let i = 0; i < count; i++) {
    const t = i / 187.5;
    const beat = (i % 94) / 94.0;
    const decay = Math.exp(-beat * 4.0);
    const melody = Math.abs(Math.sin(t * 0.5)) * 0.6 + 0.3;
    const amp = (decay * 0.7 + 0.25) * melody;
    const sampleMax = Math.round(Math.min(0.95, Math.max(0.05, amp)) * 32767);
    pairs.push([-sampleMax, sampleMax]);
  }

  return {
    sample_rate: sampleRate,
    frames: totalFrames,
    duration_secs: durationSecs,
    levels: [
      { bucket_samples: 256, pairs },
      { bucket_samples: 1024, pairs: pairs.filter((_, idx) => idx % 4 === 0) },
      { bucket_samples: 4096, pairs: pairs.filter((_, idx) => idx % 16 === 0) },
    ],
  };
}

export async function getBenchmark1HourPeaks(): Promise<WaveformPeaksDto> {
  if (isTauriEnvironment()) {
    return await invoke<WaveformPeaksDto>('get_benchmark_1hour_peaks');
  }

  // 1-hour waveform benchmark data (3600s @ 48kHz, 675,000 buckets)
  const sampleRate = 48000;
  const durationSecs = 3600.0;
  const totalFrames = sampleRate * durationSecs;
  const count = Math.floor(totalFrames / 256); // 675,000
  const pairs: [number, number][] = [];

  for (let i = 0; i < count; i++) {
    const t = i / 187.5;
    const beat = (i % 94) / 94.0;
    const decay = Math.exp(-beat * 4.0);
    const melody = Math.abs(Math.sin(t * 0.5)) * 0.6 + 0.3;
    const amp = (decay * 0.7 + 0.25) * melody;
    const sampleMax = Math.round(Math.min(0.95, Math.max(0.05, amp)) * 32767);
    pairs.push([-sampleMax, sampleMax]);
  }

  return {
    sample_rate: sampleRate,
    frames: totalFrames,
    duration_secs: durationSecs,
    levels: [
      { bucket_samples: 256, pairs },
      { bucket_samples: 1024, pairs: pairs.filter((_, idx) => idx % 4 === 0) },
      { bucket_samples: 4096, pairs: pairs.filter((_, idx) => idx % 16 === 0) },
    ],
  };
}

export async function subscribeMeters(
  onMeter: (frame: MeterFrame) => void
): Promise<() => void> {
  if (isTauriEnvironment()) {
    const channel = new Channel<MeterFrame>();
    channel.onmessage = onMeter;
    await invoke('subscribe_meters', { onEvent: channel });
    return () => {};
  }

  // Browser mock meter interval (30 Hz)
  let lastPos = mockTransport.position_secs;
  let interval = setInterval(() => {
    if (mockTransport.playing) {
      lastPos += 0.033;
      if (lastPos >= mockTransport.duration_secs) {
        if (mockTransport.loop_enabled) {
          lastPos = mockTransport.loop_start_secs;
        } else {
          lastPos = 0.0;
          mockTransport.playing = false;
        }
      }
      mockTransport.position_secs = lastPos;

      const dynamic = Math.abs(Math.sin(lastPos * 3.14159)) * 0.75 + 0.15;
      mockTransport.master_peak_l = dynamic * 0.85 * mockTransport.volume;
      mockTransport.master_peak_r = dynamic * 0.78 * mockTransport.volume;
      mockTransport.master_rms_l = mockTransport.master_peak_l * 0.65;
      mockTransport.master_rms_r = mockTransport.master_peak_r * 0.62;
    } else {
      mockTransport.master_peak_l = 0.0;
      mockTransport.master_peak_r = 0.0;
      mockTransport.master_rms_l = 0.0;
      mockTransport.master_rms_r = 0.0;
    }

    onMeter({
      peak_l: mockTransport.master_peak_l,
      peak_r: mockTransport.master_peak_r,
      rms_l: mockTransport.master_rms_l,
      rms_r: mockTransport.master_rms_r,
      position_secs: mockTransport.position_secs,
      anchor_timestamp_ms: performance.now(),
      playing: mockTransport.playing,
      duration_secs: mockTransport.duration_secs,
    });
  }, 33);

  return () => {
    clearInterval(interval);
  };
}

// --- M5 Workspace APIs & Datasets ---

export async function getAudioDevices(): Promise<AudioDevice[]> {
  if (isTauriEnvironment()) {
    try {
      return await invoke<AudioDevice[]>('get_audio_devices');
    } catch {
      // Fallback to detected defaults
    }
  }

  return [
    {
      name: 'PipeWire Audio Sink (Default)',
      is_default: true,
      direction: 'output',
      channels: 2,
      sample_rate: 48000,
      sample_format: 'f32',
    },
    {
      name: 'ALSA HDA Intel PCH (Analog Stereo)',
      is_default: false,
      direction: 'output',
      channels: 2,
      sample_rate: 44100,
      sample_format: 'i16',
    },
    {
      name: 'PipeWire Audio Capture (Default Mic)',
      is_default: true,
      direction: 'input',
      channels: 2,
      sample_rate: 48000,
      sample_format: 'f32',
    },
    {
      name: 'USB Studio Condenser Mic (Direct ALSA)',
      is_default: false,
      direction: 'input',
      channels: 1,
      sample_rate: 48000,
      sample_format: 'f32',
    },
  ];
}

export async function getModelCatalogue(): Promise<ModelItem[]> {
  if (isTauriEnvironment()) {
    try {
      return await invoke<ModelItem[]>('get_model_catalogue');
    } catch {
      // Fallback
    }
  }

  return [
    {
      id: 'kokoro-tts-v0_19',
      name: 'Kokoro TTS (Light)',
      kind: 'tts_light',
      size_mb: 84.5,
      status: 'installed',
      license: 'Apache 2.0',
      spdx: 'Apache-2.0',
      description: 'Ultra-fast lightweight speech synthesis with sub-50ms latency.',
      is_builtin: true,
    },
    {
      id: 'styletts2-medium',
      name: 'StyleTTS 2 Studio',
      kind: 'tts_medium',
      size_mb: 235.0,
      status: 'verified',
      license: 'MIT',
      spdx: 'MIT',
      description: 'High-fidelity expressive neural speech synthesis with multi-style diffusion.',
      is_builtin: false,
    },
    {
      id: 'sherpa-onnx-stt',
      name: 'Sherpa-ONNX Whisper',
      kind: 'stt',
      size_mb: 142.0,
      status: 'installed',
      license: 'Apache 2.0',
      spdx: 'Apache-2.0',
      description: 'Real-time local speech-to-text with word-level timestamps and VAD.',
      is_builtin: true,
    },
    {
      id: 'acoustic-voice-clone',
      name: 'Acoustic Voiceprint Cloner',
      kind: 'cloning',
      size_mb: 68.0,
      status: 'installed',
      license: 'Proprietary ALP Gate',
      spdx: 'LicenseRef-ALP-Consent',
      description: 'Zero-shot voice cloning engine strictly gated by human legal consent record.',
      is_builtin: true,
    },
  ];
}

let mockVoiceProfiles: VoiceProfile[] = [
  {
    id: 'voice-linus',
    name: 'Linus Sebastian',
    language: 'English',
    tags: ['tech', 'energetic', 'clear'],
    kind: 'cloned',
    speed: 1.05,
    pitch: 0.0,
    energy: 1.1,
    emotion: 'warm',
    consent: {
      speaker_name: 'Linus Sebastian',
      attested_by: 'User (Producer)',
      timestamp: Date.now() - 86400000,
      statement: 'I certify that the speaker provided explicit consent to generate this clone.',
    },
  },
  {
    id: 'voice-narrator',
    name: 'Studio Narrator (Acoustic)',
    language: 'English',
    tags: ['audiobook', 'deep', 'calm'],
    kind: 'prebuilt',
    speed: 0.95,
    pitch: -1.5,
    energy: 0.95,
    emotion: 'neutral',
  },
  {
    id: 'voice-broadcast',
    name: 'Radio Broadcaster',
    language: 'English',
    tags: ['broadcast', 'punchy', 'commercial'],
    kind: 'acoustic',
    speed: 1.1,
    pitch: 0.5,
    energy: 1.25,
    emotion: 'excited',
  },
];

export async function getVoiceProfiles(): Promise<VoiceProfile[]> {
  if (isTauriEnvironment()) {
    try {
      return await invoke<VoiceProfile[]>('get_voice_profiles');
    } catch {
      // Fallback
    }
  }
  return [...mockVoiceProfiles];
}

export async function createVoiceProfile(
  profile: Partial<VoiceProfile>,
  consent: ConsentRecord
): Promise<VoiceProfile> {
  // Validate consent gate strictly
  if (!consent.speaker_name.trim() || !consent.attested_by.trim() || consent.statement.length < 10) {
    throw new Error('Consent gate violation: valid human consent record is mandatory.');
  }

  const newProfile: VoiceProfile = {
    id: `voice-${Date.now()}`,
    name: profile.name || 'New Voice',
    language: profile.language || 'English',
    tags: profile.tags || ['custom'],
    kind: 'cloned',
    speed: profile.speed || 1.0,
    pitch: profile.pitch || 0.0,
    energy: profile.energy || 1.0,
    emotion: profile.emotion || 'neutral',
    consent,
  };

  mockVoiceProfiles.unshift(newProfile);
  return newProfile;
}

export async function getEffectsChain(): Promise<EffectStage[]> {
  return [
    {
      id: 'hum_removal',
      name: '50/60Hz Hum Filter',
      category: 'Denoise',
      enabled: true,
      params: { freq: 60, q: 8.0, reduction: -24 },
    },
    {
      id: 'click_removal',
      name: 'De-Clicker & De-Plosive',
      category: 'Restoration',
      enabled: true,
      params: { threshold: -18, sensitivity: 0.75 },
    },
    {
      id: 'parametric_eq',
      name: '4-Band Parametric EQ',
      category: 'Tone',
      enabled: true,
      params: { lowGain: -1.5, midGain: 2.0, highGain: 1.0, highFreq: 8500 },
    },
    {
      id: 'de_esser',
      name: 'Dynamic De-Esser',
      category: 'Vocal',
      enabled: true,
      params: { threshold: -22, frequency: 6500, ratio: 4.0 },
    },
    {
      id: 'compressor',
      name: 'Multiband Compressor',
      category: 'Dynamics',
      enabled: true,
      params: { threshold: -16, ratio: 3.5, attack: 15, release: 120 },
    },
    {
      id: 'peak_limiter',
      name: 'Broadcast Peak Limiter',
      category: 'Mastering',
      enabled: true,
      params: { ceiling: -1.0, release: 50 },
    },
  ];
}

export async function getDiscoveredPlugins(): Promise<DiscoveredPlugin[]> {
  return [
    {
      id: 'u-he-satin',
      name: 'Satin Tape Machine',
      vendor: 'u-he',
      format: 'CLAP',
      category: 'Saturation',
    },
    {
      id: 'valhalla-vintage-verb',
      name: 'VintageVerb Studio',
      vendor: 'Valhalla DSP',
      format: 'CLAP',
      category: 'Reverb',
    },
    {
      id: 'lsp-compressor-stereo',
      name: 'LSP Multiband Compressor',
      vendor: 'Linux Studio Plugins',
      format: 'LV2',
      category: 'Dynamics',
    },
    {
      id: 'calf-parametric-eq',
      name: 'Calf 8-Band Parametric EQ',
      vendor: 'Calf Studio Gear',
      format: 'LV2',
      category: 'Equalizer',
    },
  ];
}

export async function getCaptures(): Promise<CaptureTake[]> {
  return [
    {
      id: 'take-01',
      name: 'Lead Vocal — Verse 1 (Take 1)',
      duration_secs: 18.4,
      timestamp: 'Today at 11:42 AM',
      peak_db: -2.4,
      rms_db: -16.8,
      clipped: false,
    },
    {
      id: 'take-02',
      name: 'Lead Vocal — Verse 1 (Take 2)',
      duration_secs: 19.1,
      timestamp: 'Today at 11:46 AM',
      peak_db: -0.8,
      rms_db: -15.2,
      clipped: false,
    },
    {
      id: 'take-03',
      name: 'ADR Dub Dialogue — Cue 04',
      duration_secs: 7.8,
      timestamp: 'Today at 12:15 PM',
      peak_db: -3.1,
      rms_db: -18.2,
      clipped: false,
    },
  ];
}

export async function getTranscript(): Promise<TranscriptWord[]> {
  return [
    { word: 'Welcome', start_secs: 0.2, end_secs: 0.7, confidence: 0.99, speaker: 'Linus' },
    { word: 'to', start_secs: 0.75, end_secs: 0.9, confidence: 0.98, speaker: 'Linus' },
    { word: 'Audio', start_secs: 0.95, end_secs: 1.4, confidence: 0.99, speaker: 'Linus' },
    { word: 'Labs', start_secs: 1.45, end_secs: 1.85, confidence: 0.99, speaker: 'Linus' },
    { word: 'Pro,', start_secs: 1.9, end_secs: 2.3, confidence: 0.97, speaker: 'Linus' },
    { word: 'the', start_secs: 2.6, end_secs: 2.8, confidence: 0.96, speaker: 'Linus' },
    { word: 'open-source', start_secs: 2.85, end_secs: 3.6, confidence: 0.98, speaker: 'Linus' },
    { word: 'studio', start_secs: 3.65, end_secs: 4.1, confidence: 0.99, speaker: 'Linus' },
    { word: 'workstation', start_secs: 4.15, end_secs: 4.9, confidence: 0.98, speaker: 'Linus' },
    { word: 'engineered', start_secs: 5.1, end_secs: 5.8, confidence: 0.99, speaker: 'Linus' },
    { word: 'for', start_secs: 5.85, end_secs: 6.0, confidence: 0.99, speaker: 'Linus' },
    { word: 'Linux.', start_secs: 6.05, end_secs: 6.6, confidence: 0.99, speaker: 'Linus' },
  ];
}

export async function getStories(): Promise<StoryScript[]> {
  return [
    {
      id: 'story-01',
      title: 'The AI Audio Revolution',
      chapter: 1,
      lines: [
        {
          id: 'l-01',
          speaker: 'Studio Narrator (Acoustic)',
          text: 'In the heart of the digital workstation, silence gave way to sound.',
          status: 'ready',
          take_duration: 4.8,
        },
        {
          id: 'l-02',
          speaker: 'Linus Sebastian',
          text: 'We are finally building studio audio tools that do not rely on cloud servers or closed silos.',
          status: 'ready',
          take_duration: 6.2,
        },
        {
          id: 'l-03',
          speaker: 'Radio Broadcaster',
          text: 'And the audio latency? Zero buffer underruns at forty-eight kilohertz!',
          status: 'pending',
        },
      ],
    },
  ];
}
