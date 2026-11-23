/**
 * Audio Labs Pro - Tauri IPC Type Contract
 *
 * Generated/synchronized with alp_session and alp_api models.
 * Used for typed communication between React frontend and alp-desktop.
 */

export interface ProjectSummary {
  id: string;
  name: string;
  revision: number;
  sample_rate: number;
  duration_secs: number;
  track_count: number;
  clip_count: number;
  undo_steps: number;
  redo_steps: number;
  is_dirty: boolean;
}

export interface TrackSummary {
  id: string;
  name: string;
  index: number;
  gain: number;
  pan: number;
  muted: boolean;
  solo: boolean;
  armed: boolean;
  clip_count: number;
}

export interface PanelState {
  id: string;
  kind: string;
  dock_side: string;
  visible: boolean;
  floating: boolean;
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface WorkspaceState {
  version: number;
  theme: string;
  reduce_motion: boolean;
  glass: string;
  panels: PanelState[];
  first_run_completed: boolean;
}

export interface SessionRevisionPayload {
  revision: number;
  is_dirty: boolean;
  reason: string;
}

export interface IpcError {
  code: string;
  message: string;
}
