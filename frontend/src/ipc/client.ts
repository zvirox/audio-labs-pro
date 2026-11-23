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
