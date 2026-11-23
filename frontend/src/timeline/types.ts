/**
 * Audio Labs Pro - Multitrack Timeline Data Types
 *
 * Framework-free data structures matching the Rust alp_session::timeline contract,
 * along with hit-testing and drag interaction representations.
 */

export interface TimelineDto {
  sample_rate: number;
  duration_secs: number;
  tracks: TimelineTrackDto[];
  markers: TimelineMarkerDto[];
  loop_range: TimelineLoopRangeDto | null;
  selection: TimelineSelectionDto | null;
  selected_clip_id: string | null;
  can_undo: boolean;
  can_redo: boolean;
  undo_steps: number;
  redo_steps: number;
  revision: number;
}

export interface TimelineTrackDto {
  id: string;
  name: string;
  gain: number;
  pan: number;
  muted: boolean;
  solo: boolean;
  armed: boolean;
  clips: TimelineClipDto[];
}

export interface TimelineClipDto {
  id: string;
  source_id: string;
  name: string;
  timeline_start_secs: number;
  duration_secs: number;
  source_start_secs: number;
  gain: number;
  fade_in_secs: number;
  fade_out_secs: number;
  envelope: TimelineEnvelopePointDto[];
}

export interface TimelineEnvelopePointDto {
  offset_secs: number;
  gain: number;
}

export interface TimelineMarkerDto {
  id: string;
  time_secs: number;
  label: string;
}

export interface TimelineLoopRangeDto {
  start_secs: number;
  end_secs: number;
}

export interface TimelineSelectionDto {
  track_id: string | null;
  start_secs: number;
  end_secs: number;
}

export interface ViewportConfig {
  zoom: number; // pixels per second
  scrollX: number; // pixels scrolled horizontally
  scrollY: number; // pixels scrolled vertically
  trackHeight: number; // height of each track lane in pixels
  rulerHeight: number; // height of timecode ruler in pixels
  trackHeaderWidth: number; // width of track lane headers in pixels
  width: number; // canvas viewport width
  height: number; // canvas viewport height
}

export type HitTargetKind =
  | 'none'
  | 'ruler'
  | 'playhead'
  | 'marker'
  | 'loop_start'
  | 'loop_end'
  | 'clip_body'
  | 'clip_trim_left'
  | 'clip_trim_right'
  | 'clip_fade_in'
  | 'clip_fade_out'
  | 'track_empty';

export interface HitTarget {
  kind: HitTargetKind;
  timeSecs: number;
  trackIndex: number;
  trackId: string | null;
  clip: TimelineClipDto | null;
  marker: TimelineMarkerDto | null;
}

export type DragMode =
  | 'idle'
  | 'scrub_playhead'
  | 'move_clip'
  | 'trim_clip_left'
  | 'trim_clip_right'
  | 'fade_in'
  | 'fade_out'
  | 'drag_loop_start'
  | 'drag_loop_end'
  | 'range_select';

export interface DragState {
  mode: DragMode;
  startX: number;
  startY: number;
  startTimeSecs: number;
  currentX: number;
  currentY: number;
  currentTimeSecs: number;
  snappedTimeSecs: number;
  target: HitTarget;
}

export type TimelineCommandPayload =
  | { type: 'seek'; positionSecs: number }
  | { type: 'move_clip'; clipId: string; targetTrackId: string; newStartSecs: number }
  | { type: 'trim_clip'; clipId: string; newStartSecs: number; newDurationSecs: number }
  | { type: 'set_clip_fades'; clipId: string; fadeInSecs: number; fadeOutSecs: number }
  | { type: 'set_loop_range'; startSecs: number; endSecs: number; enabled: boolean }
  | { type: 'set_selection'; startSecs: number; endSecs: number }
  | { type: 'select_clip'; clipId: string | null };
