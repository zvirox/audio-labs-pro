/**
 * Audio Labs Pro - Tauri IPC Type Contract
 *
 * Generated and synchronized with alp_session and alp_api models.
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

export interface PanelRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export type DockSide = 'left' | 'right';

export type PanelKind =
  | 'mixer'
  | 'transport'
  | 'timeline'
  | 'inspector'
  | 'media_bin'
  | 'voice_lab'
  | 'effects'
  | 'models'
  | 'export_queue'
  | 'ai_activity';

export interface PanelState {
  kind: PanelKind;
  rect: PanelRect;
  z: number;
  collapsed: boolean;
  docked: DockSide | null;
  open: boolean;
}

export interface WorkspaceState {
  version: number;
  theme: string;
  reduce_motion: boolean;
  glass: string;
  panels: PanelState[];
  approved_ai_clients?: string[];
  first_run_completed: boolean;
}

export interface GlassReport {
  session: string;
  desktop: string;
  handle: string;
  mode: 'compositor_blur' | 'tint_fallback';
  blur_available: boolean;
  detail: string;
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

export interface MeterFrame {
  peak_l: number;
  peak_r: number;
  rms_l: number;
  rms_r: number;
  position_secs: number;
  anchor_timestamp_ms: number;
  playing: boolean;
  duration_secs: number;
}

export interface TransportState {
  playing: boolean;
  position_secs: number;
  position_frames: number;
  duration_secs: number;
  sample_rate: number;
  loop_enabled: boolean;
  loop_start_secs: number;
  loop_end_secs: number;
  volume: number;
  master_peak_l: number;
  master_peak_r: number;
  master_rms_l: number;
  master_rms_r: number;
  anchor_timestamp_ms: number;
}

export interface PeakLevelDto {
  bucket_samples: number;
  pairs: [number, number][];
}

export interface WaveformPeaksDto {
  sample_rate: number;
  frames: number;
  duration_secs: number;
  levels: PeakLevelDto[];
}
