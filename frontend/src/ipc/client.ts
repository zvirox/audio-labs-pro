/**
 * Audio Labs Pro - Tauri IPC Client
 *
 * Provides typed invoke wrappers and event listeners.
 * Seamlessly provides development mock fallbacks when previewed outside Tauri runtime.
 */

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  ProjectSummary,
  WorkspaceState,
  GlassReport,
  SessionRevisionPayload,
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
