/**
 * Audio Labs Pro - Timeline Mathematics & Hit-Testing
 *
 * Framework-free geometric and temporal calculations for multitrack timeline editing.
 * Zero UI or React dependencies; suitable for direct unit testing in Node/browser.
 */

import type {
  TimelineDto,
  TimelineClipDto,
  ViewportConfig,
  HitTarget,
} from './types';

/** Convert time in seconds to horizontal canvas pixel offset */
export function timeToPixels(
  timeSecs: number,
  zoom: number,
  scrollX: number,
  headerWidth: number = 0
): number {
  return headerWidth + (timeSecs * zoom) - scrollX;
}

/** Convert horizontal canvas pixel offset to time in seconds */
export function pixelsToTime(
  px: number,
  zoom: number,
  scrollX: number,
  headerWidth: number = 0
): number {
  if (zoom <= 0) return 0;
  const time = (px - headerWidth + scrollX) / zoom;
  return Math.max(0, time);
}

/** Format seconds into standard studio timecode `MM:SS.mmm` */
export function formatTimecode(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return '00:00.000';
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  const millis = Math.floor((seconds % 1) * 1000);
  return `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}.${millis.toString().padStart(3, '0')}`;
}

/** Find magnetic snap target from candidate timestamps within threshold */
export function findSnapTarget(
  timeSecs: number,
  candidates: number[],
  thresholdSecs: number = 0.05
): { time: number; snapped: boolean } {
  let closestDist = Infinity;
  let bestTime = timeSecs;
  let didSnap = false;

  for (const candidate of candidates) {
    const dist = Math.abs(candidate - timeSecs);
    if (dist <= thresholdSecs && dist < closestDist) {
      closestDist = dist;
      bestTime = candidate;
      didSnap = true;
    }
  }

  return { time: bestTime, snapped: didSnap };
}

/** Calculate which track lane corresponds to a vertical canvas Y offset */
export function trackIndexAtY(
  y: number,
  trackHeight: number,
  rulerHeight: number,
  scrollY: number,
  trackCount: number
): number {
  if (trackCount <= 0 || trackHeight <= 0) return -1;
  const relativeY = y - rulerHeight + scrollY;
  if (relativeY < 0) return -1;
  const index = Math.floor(relativeY / trackHeight);
  return index >= 0 && index < trackCount ? index : -1;
}

/** Perform precise hit testing against ruler, markers, loops, clips, handles, and tracks */
export function hitTest(
  x: number,
  y: number,
  timeline: TimelineDto,
  viewport: ViewportConfig
): HitTarget {
  const { zoom, scrollX, scrollY, trackHeight, rulerHeight, trackHeaderWidth } = viewport;
  const timeAtCursor = pixelsToTime(x, zoom, scrollX, trackHeaderWidth);

  // 1. Time Ruler Hit Zone
  if (y <= rulerHeight) {
    if (x < trackHeaderWidth) {
      return {
        kind: 'none',
        timeSecs: 0,
        trackIndex: -1,
        trackId: null,
        clip: null,
        marker: null,
      };
    }

    // Check marker handle collisions (±8px threshold)
    for (const marker of timeline.markers) {
      const markerX = timeToPixels(marker.time_secs, zoom, scrollX, trackHeaderWidth);
      if (Math.abs(x - markerX) <= 8) {
        return {
          kind: 'marker',
          timeSecs: marker.time_secs,
          trackIndex: -1,
          trackId: null,
          clip: null,
          marker,
        };
      }
    }

    // Check loop boundaries if enabled
    if (timeline.loop_range) {
      const loopStartX = timeToPixels(timeline.loop_range.start_secs, zoom, scrollX, trackHeaderWidth);
      const loopEndX = timeToPixels(timeline.loop_range.end_secs, zoom, scrollX, trackHeaderWidth);
      if (Math.abs(x - loopStartX) <= 6) {
        return {
          kind: 'loop_start',
          timeSecs: timeline.loop_range.start_secs,
          trackIndex: -1,
          trackId: null,
          clip: null,
          marker: null,
        };
      }
      if (Math.abs(x - loopEndX) <= 6) {
        return {
          kind: 'loop_end',
          timeSecs: timeline.loop_range.end_secs,
          trackIndex: -1,
          trackId: null,
          clip: null,
          marker: null,
        };
      }
    }

    // Default ruler click is playhead seek/scrub
    return {
      kind: 'ruler',
      timeSecs: timeAtCursor,
      trackIndex: -1,
      trackId: null,
      clip: null,
      marker: null,
    };
  }

  // 2. Track Lanes Hit Zone
  if (x < trackHeaderWidth) {
    return {
      kind: 'none',
      timeSecs: 0,
      trackIndex: -1,
      trackId: null,
      clip: null,
      marker: null,
    };
  }

  const trackIndex = trackIndexAtY(y, trackHeight, rulerHeight, scrollY, timeline.tracks.length);
  if (trackIndex < 0 || trackIndex >= timeline.tracks.length) {
    return {
      kind: 'none',
      timeSecs: timeAtCursor,
      trackIndex: -1,
      trackId: null,
      clip: null,
      marker: null,
    };
  }

  const track = timeline.tracks[trackIndex];
  const trackTopY = rulerHeight + (trackIndex * trackHeight) - scrollY;
  const relativeYInTrack = y - trackTopY;

  // Check clips in this track
  const HANDLE_WIDTH = 8;
  const FADE_HANDLE_SIZE = 14;

  for (const clip of track.clips) {
    const clipStartX = timeToPixels(clip.timeline_start_secs, zoom, scrollX, trackHeaderWidth);
    const clipWidth = Math.max(1, clip.duration_secs * zoom);
    const clipEndX = clipStartX + clipWidth;

    if (x >= clipStartX && x <= clipEndX && relativeYInTrack >= 4 && relativeYInTrack <= trackHeight - 4) {
      // Top-left fade-in handle
      const fadeInHandleX = clipStartX + (clip.fade_in_secs * zoom);
      if (
        relativeYInTrack <= FADE_HANDLE_SIZE &&
        Math.abs(x - fadeInHandleX) <= FADE_HANDLE_SIZE
      ) {
        return {
          kind: 'clip_fade_in',
          timeSecs: timeAtCursor,
          trackIndex,
          trackId: track.id,
          clip,
          marker: null,
        };
      }

      // Top-right fade-out handle
      const fadeOutHandleX = clipEndX - (clip.fade_out_secs * zoom);
      if (
        relativeYInTrack <= FADE_HANDLE_SIZE &&
        Math.abs(x - fadeOutHandleX) <= FADE_HANDLE_SIZE
      ) {
        return {
          kind: 'clip_fade_out',
          timeSecs: timeAtCursor,
          trackIndex,
          trackId: track.id,
          clip,
          marker: null,
        };
      }

      // Left trim edge
      if (x <= clipStartX + HANDLE_WIDTH) {
        return {
          kind: 'clip_trim_left',
          timeSecs: timeAtCursor,
          trackIndex,
          trackId: track.id,
          clip,
          marker: null,
        };
      }

      // Right trim edge
      if (x >= clipEndX - HANDLE_WIDTH) {
        return {
          kind: 'clip_trim_right',
          timeSecs: timeAtCursor,
          trackIndex,
          trackId: track.id,
          clip,
          marker: null,
        };
      }

      // Inside clip body
      return {
        kind: 'clip_body',
        timeSecs: timeAtCursor,
        trackIndex,
        trackId: track.id,
        clip,
        marker: null,
      };
    }
  }

  // Empty space on this track
  return {
    kind: 'track_empty',
    timeSecs: timeAtCursor,
    trackIndex,
    trackId: track.id,
    clip: null,
    marker: null,
  };
}

/** Calculate non-destructive clip trim boundaries */
export function calculateTrim(
  initialClip: TimelineClipDto,
  isLeftTrim: boolean,
  deltaSecs: number,
  minDurationSecs: number = 0.05
): { newStartSecs: number; newDurationSecs: number } {
  if (isLeftTrim) {
    const newStart = Math.max(0, initialClip.timeline_start_secs + deltaSecs);
    const newDuration = Math.max(minDurationSecs, initialClip.duration_secs - (newStart - initialClip.timeline_start_secs));
    return { newStartSecs: newStart, newDurationSecs: newDuration };
  } else {
    const newDuration = Math.max(minDurationSecs, initialClip.duration_secs + deltaSecs);
    return { newStartSecs: initialClip.timeline_start_secs, newDurationSecs: newDuration };
  }
}

/** Calculate updated fade-in and fade-out durations */
export function calculateFades(
  initialClip: TimelineClipDto,
  isFadeIn: boolean,
  deltaSecs: number
): { newFadeInSecs: number; newFadeOutSecs: number } {
  let fadeIn = initialClip.fade_in_secs;
  let fadeOut = initialClip.fade_out_secs;
  const maxFade = initialClip.duration_secs;

  if (isFadeIn) {
    fadeIn = Math.max(0, Math.min(maxFade - fadeOut, fadeIn + deltaSecs));
  } else {
    fadeOut = Math.max(0, Math.min(maxFade - fadeIn, fadeOut + deltaSecs));
  }

  return { newFadeInSecs: fadeIn, newFadeOutSecs: fadeOut };
}
