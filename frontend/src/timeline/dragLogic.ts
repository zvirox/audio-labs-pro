/**
 * Audio Labs Pro - Drag Interaction Logic
 *
 * Framework-free drag state machine for timeline scrubbing, clip movement,
 * trimming, fade adjustments, loop boundaries, and selection.
 */

import type {
  DragMode,
  DragState,
  HitTarget,
  TimelineCommandPayload,
} from './types';
import {
  pixelsToTime,
  findSnapTarget,
  calculateTrim,
  calculateFades,
} from './timelineMath';

/** Initialize a new drag session from a pointerdown hit test result */
export function createDragSession(
  downX: number,
  downY: number,
  target: HitTarget,
  zoom: number,
  scrollX: number,
  headerWidth: number,
  altKey: boolean = false
): DragState {
  const initialTime = pixelsToTime(downX, zoom, scrollX, headerWidth);
  let mode: DragMode = 'idle';

  switch (target.kind) {
    case 'ruler':
      mode = 'scrub_playhead';
      break;
    case 'clip_body':
      mode = altKey ? 'slip_clip' : 'move_clip';
      break;
    case 'clip_trim_left':
      mode = 'trim_clip_left';
      break;
    case 'clip_trim_right':
      mode = 'trim_clip_right';
      break;
    case 'clip_fade_in':
      mode = 'fade_in';
      break;
    case 'clip_fade_out':
      mode = 'fade_out';
      break;
    case 'loop_start':
      mode = 'drag_loop_start';
      break;
    case 'loop_end':
      mode = 'drag_loop_end';
      break;
    case 'track_empty':
      mode = 'range_select';
      break;
    default:
      mode = 'idle';
      break;
  }

  return {
    mode,
    startX: downX,
    startY: downY,
    startTimeSecs: initialTime,
    currentX: downX,
    currentY: downY,
    currentTimeSecs: initialTime,
    snappedTimeSecs: initialTime,
    target,
  };
}

/** Update the drag state on pointer move with optional magnetic snapping */
export function updateDragSession(
  session: DragState,
  currentX: number,
  currentY: number,
  snapCandidates: number[],
  zoom: number,
  scrollX: number,
  headerWidth: number,
  snapThresholdPx: number = 8
): DragState {
  const currentTime = pixelsToTime(currentX, zoom, scrollX, headerWidth);
  const snapThresholdSecs = zoom > 0 ? snapThresholdPx / zoom : 0.05;
  const { time: snappedTime } = findSnapTarget(currentTime, snapCandidates, snapThresholdSecs);

  return {
    ...session,
    currentX,
    currentY,
    currentTimeSecs: currentTime,
    snappedTimeSecs: snappedTime,
  };
}

/** Convert a finished drag session into an authoritative IPC command payload */
export function resolveDragCommand(session: DragState): TimelineCommandPayload | null {
  const { mode, startTimeSecs, snappedTimeSecs, target } = session;
  const timeDelta = snappedTimeSecs - startTimeSecs;

  switch (mode) {
    case 'scrub_playhead':
      return {
        type: 'seek',
        positionSecs: Math.max(0, snappedTimeSecs),
      };

    case 'move_clip': {
      if (!target.clip || !target.trackId) return null;
      const newStart = Math.max(0, target.clip.timeline_start_secs + timeDelta);
      return {
        type: 'move_clip',
        clipId: target.clip.id,
        targetTrackId: target.trackId,
        newStartSecs: newStart,
      };
    }

    case 'slip_clip': {
      if (!target.clip) return null;
      return {
        type: 'slip_clip',
        clipId: target.clip.id,
        deltaSecs: timeDelta,
      };
    }

    case 'trim_clip_left': {
      if (!target.clip) return null;
      const { newStartSecs, newDurationSecs } = calculateTrim(target.clip, true, timeDelta);
      return {
        type: 'trim_clip',
        clipId: target.clip.id,
        newStartSecs,
        newDurationSecs,
      };
    }

    case 'trim_clip_right': {
      if (!target.clip) return null;
      const { newStartSecs, newDurationSecs } = calculateTrim(target.clip, false, timeDelta);
      return {
        type: 'trim_clip',
        clipId: target.clip.id,
        newStartSecs,
        newDurationSecs,
      };
    }

    case 'fade_in': {
      if (!target.clip) return null;
      const { newFadeInSecs, newFadeOutSecs } = calculateFades(target.clip, true, timeDelta);
      return {
        type: 'set_clip_fades',
        clipId: target.clip.id,
        fadeInSecs: newFadeInSecs,
        fadeOutSecs: newFadeOutSecs,
      };
    }

    case 'fade_out': {
      if (!target.clip) return null;
      const { newFadeInSecs, newFadeOutSecs } = calculateFades(target.clip, false, -timeDelta);
      return {
        type: 'set_clip_fades',
        clipId: target.clip.id,
        fadeInSecs: newFadeInSecs,
        fadeOutSecs: newFadeOutSecs,
      };
    }

    case 'drag_loop_start':
      return {
        type: 'set_loop_range',
        startSecs: Math.max(0, snappedTimeSecs),
        endSecs: Math.max(snappedTimeSecs + 0.5, startTimeSecs + 2.0),
        enabled: true,
      };

    case 'drag_loop_end':
      return {
        type: 'set_loop_range',
        startSecs: Math.max(0, startTimeSecs - 2.0),
        endSecs: Math.max(startTimeSecs, snappedTimeSecs),
        enabled: true,
      };

    case 'range_select': {
      const start = Math.min(startTimeSecs, snappedTimeSecs);
      const end = Math.max(startTimeSecs, snappedTimeSecs);
      return {
        type: 'set_selection',
        startSecs: Math.max(0, start),
        endSecs: Math.max(start, end),
      };
    }

    default:
      return null;
  }
}
