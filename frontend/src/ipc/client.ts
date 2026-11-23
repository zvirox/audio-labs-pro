/**
 * Audio Labs Pro - Tauri IPC Client
 *
 * Provides typed invoke wrappers and event listeners.
 * Seamlessly provides development mock fallbacks when previewed outside Tauri runtime.
 */

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { ProjectSummary, WorkspaceState, SessionRevisionPayload } from './types';

export const isTauriEnvironment = (): boolean => {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
};

// In-memory demo state for browser mock mode
let mockRevision = 1;
let mockDirty = false;

export async function getProjectSummary(): Promise<ProjectSummary> {
  if (isTauriEnvironment()) {
    return await invoke<ProjectSummary>('get_project_summary');
  }

  // Realistic mock data matching alp_session::EditSession::demo()
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

  return {
    version: 1,
    theme: 'dark',
    reduce_motion: false,
    glass: 'compositor_blur',
    panels: [
      {
        id: 'timeline',
        kind: 'timeline',
        dock_side: 'center',
        visible: true,
        floating: false,
        x: 56,
        y: 40,
        width: 900,
        height: 480,
      },
      {
        id: 'inspector',
        kind: 'inspector',
        dock_side: 'right',
        visible: true,
        floating: false,
        x: 956,
        y: 40,
        width: 280,
        height: 600,
      },
    ],
    first_run_completed: true,
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

  // Dispatch custom event in browser mock mode
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

  // Browser mock listener
  const listener = (event: Event) => {
    const custom = event as CustomEvent<SessionRevisionPayload>;
    handler(custom.detail);
  };
  window.addEventListener('session_revision_changed', listener);
  return () => {
    window.removeEventListener('session_revision_changed', listener);
  };
}
