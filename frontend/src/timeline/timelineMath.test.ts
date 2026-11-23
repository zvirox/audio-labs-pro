/**
 * Audio Labs Pro - Timeline Math & Drag Logic Unit Tests
 *
 * Direct Node test suite validating coordinate projections, hit-testing accuracy,
 * magnetic snapping, trim/fade boundaries, and IPC command resolution.
 */

import test from 'node:test';
import assert from 'node:assert';

import {
  timeToPixels,
  pixelsToTime,
  formatTimecode,
  findSnapTarget,
  trackIndexAtY,
  hitTest,
  calculateTrim,
  calculateFades,
} from './timelineMath';
import {
  createDragSession,
  updateDragSession,
  resolveDragCommand,
} from './dragLogic';
import type { TimelineDto, ViewportConfig, TimelineClipDto } from './types';

// Deterministic test timeline fixture
const mockTimeline: TimelineDto = {
  sample_rate: 48000,
  duration_secs: 20.0,
  tracks: [
    {
      id: 'track-01',
      name: 'Vocal Track 1',
      gain: 1.0,
      pan: 0.0,
      muted: false,
      solo: false,
      armed: false,
      clips: [
        {
          id: 'clip-01',
          source_id: 'src-01',
          name: 'Take 1',
          timeline_start_secs: 2.0,
          duration_secs: 6.0,
          source_start_secs: 0.0,
          gain: 1.0,
          fade_in_secs: 0.5,
          fade_out_secs: 0.5,
          envelope: [],
        },
      ],
    },
    {
      id: 'track-02',
      name: 'Instrumental',
      gain: 0.8,
      pan: 0.0,
      muted: false,
      solo: false,
      armed: false,
      clips: [],
    },
  ],
  markers: [
    { id: 'marker-01', time_secs: 4.0, label: 'Chorus' },
  ],
  loop_range: {
    start_secs: 1.0,
    end_secs: 9.0,
  },
  selection: null,
  selected_clip_id: 'clip-01',
  can_undo: true,
  can_redo: false,
  undo_steps: 3,
  redo_steps: 0,
  revision: 12,
};

const mockViewport: ViewportConfig = {
  zoom: 100, // 100px per second
  scrollX: 0,
  scrollY: 0,
  trackHeight: 80,
  rulerHeight: 32,
  trackHeaderWidth: 160,
  width: 1200,
  height: 600,
};

test('timeToPixels and pixelsToTime roundtrip coordinate identity', () => {
  const zoom = 120;
  const scrollX = 50;
  const headerWidth = 160;

  const originalSecs = 14.25;
  const px = timeToPixels(originalSecs, zoom, scrollX, headerWidth);
  const recoveredSecs = pixelsToTime(px, zoom, scrollX, headerWidth);

  assert(Math.abs(originalSecs - recoveredSecs) < 1e-9, 'Time must round-trip accurately');
});

test('formatTimecode handles edge cases and standard formats', () => {
  assert.strictEqual(formatTimecode(0), '00:00.000');
  assert.strictEqual(formatTimecode(65.5), '01:05.500');
  assert.strictEqual(formatTimecode(-5), '00:00.000');
  assert.strictEqual(formatTimecode(NaN), '00:00.000');
});

test('findSnapTarget snaps within threshold and ignores outside', () => {
  const candidates = [2.0, 4.0, 8.0];
  const snapThreshold = 0.05;

  // Within threshold: 2.03 -> snaps to 2.0
  const snap1 = findSnapTarget(2.03, candidates, snapThreshold);
  assert.strictEqual(snap1.snapped, true);
  assert.strictEqual(snap1.time, 2.0);

  // Outside threshold: 2.15 -> remains 2.15
  const snap2 = findSnapTarget(2.15, candidates, snapThreshold);
  assert.strictEqual(snap2.snapped, false);
  assert.strictEqual(snap2.time, 2.15);
});

test('trackIndexAtY calculates vertical track lane indices correctly', () => {
  const trackHeight = 80;
  const rulerHeight = 32;
  const trackCount = 4;

  // In ruler area
  assert.strictEqual(trackIndexAtY(20, trackHeight, rulerHeight, 0, trackCount), -1);

  // In Track 0 (Y from 32 to 112)
  assert.strictEqual(trackIndexAtY(40, trackHeight, rulerHeight, 0, trackCount), 0);
  assert.strictEqual(trackIndexAtY(110, trackHeight, rulerHeight, 0, trackCount), 0);

  // In Track 1 (Y from 112 to 192)
  assert.strictEqual(trackIndexAtY(120, trackHeight, rulerHeight, 0, trackCount), 1);

  // Past all tracks
  assert.strictEqual(trackIndexAtY(500, trackHeight, rulerHeight, 0, trackCount), -1);
});

test('hitTest accurately detects ruler, markers, loops, handles, and clips', () => {
  // 1. Ruler hit at X = 300 (time = 1.4s), Y = 15
  const rulerHit = hitTest(300, 15, mockTimeline, mockViewport);
  assert.strictEqual(rulerHit.kind, 'ruler');

  // 2. Marker hit at marker-01 (time = 4.0s, X = 160 + 400 = 560px), Y = 15
  const markerHit = hitTest(560, 15, mockTimeline, mockViewport);
  assert.strictEqual(markerHit.kind, 'marker');
  assert.strictEqual(markerHit.marker?.id, 'marker-01');

  // 3. Loop start hit (time = 1.0s, X = 160 + 100 = 260px), Y = 15
  const loopStartHit = hitTest(260, 15, mockTimeline, mockViewport);
  assert.strictEqual(loopStartHit.kind, 'loop_start');

  // Clip 01 is on Track 0 (Y from 32 to 112).
  // Starts at 2.0s (X = 160 + 200 = 360px), ends at 8.0s (X = 160 + 800 = 960px).

  // 4. Left trim handle (X = 362, Y = 60)
  const leftTrimHit = hitTest(362, 60, mockTimeline, mockViewport);
  assert.strictEqual(leftTrimHit.kind, 'clip_trim_left');
  assert.strictEqual(leftTrimHit.clip?.id, 'clip-01');

  // 5. Right trim handle (X = 958, Y = 60)
  const rightTrimHit = hitTest(958, 60, mockTimeline, mockViewport);
  assert.strictEqual(rightTrimHit.kind, 'clip_trim_right');
  assert.strictEqual(rightTrimHit.clip?.id, 'clip-01');

  // 6. Fade-in handle (X = 360 + 50 = 410, Y = 36)
  const fadeInHit = hitTest(410, 36, mockTimeline, mockViewport);
  assert.strictEqual(fadeInHit.kind, 'clip_fade_in');

  // 7. Clip body center (X = 600, Y = 60)
  const bodyHit = hitTest(600, 60, mockTimeline, mockViewport);
  assert.strictEqual(bodyHit.kind, 'clip_body');
  assert.strictEqual(bodyHit.clip?.id, 'clip-01');

  // 8. Empty space on Track 1 (Y = 150, X = 400)
  const emptyHit = hitTest(400, 150, mockTimeline, mockViewport);
  assert.strictEqual(emptyHit.kind, 'track_empty');
  assert.strictEqual(emptyHit.trackIndex, 1);
});

test('calculateTrim adjusts clip boundaries without negative durations', () => {
  const clip: TimelineClipDto = {
    id: 'test',
    source_id: 's',
    name: 'test',
    timeline_start_secs: 5.0,
    duration_secs: 4.0,
    source_start_secs: 0.0,
    gain: 1.0,
    fade_in_secs: 0.0,
    fade_out_secs: 0.0,
    envelope: [],
  };

  // Left trim forward by 1.0s
  const trimLeft = calculateTrim(clip, true, 1.0);
  assert.strictEqual(trimLeft.newStartSecs, 6.0);
  assert.strictEqual(trimLeft.newDurationSecs, 3.0);

  // Right trim backward by 1.5s
  const trimRight = calculateTrim(clip, false, -1.5);
  assert.strictEqual(trimRight.newStartSecs, 5.0);
  assert.strictEqual(trimRight.newDurationSecs, 2.5);

  // Over-trim is clamped to minimum duration
  const overTrim = calculateTrim(clip, false, -10.0, 0.05);
  assert.strictEqual(overTrim.newDurationSecs, 0.05);
});

test('calculateFades updates fades bounded within clip duration', () => {
  const clip: TimelineClipDto = {
    id: 'test',
    source_id: 's',
    name: 'test',
    timeline_start_secs: 0.0,
    duration_secs: 3.0,
    source_start_secs: 0.0,
    gain: 1.0,
    fade_in_secs: 0.5,
    fade_out_secs: 0.5,
    envelope: [],
  };

  const fades = calculateFades(clip, true, 0.25);
  assert.strictEqual(fades.newFadeInSecs, 0.75);
  assert.strictEqual(fades.newFadeOutSecs, 0.5);
});

test('drag session lifecycle produces valid IPC commands', () => {
  const target = hitTest(600, 60, mockTimeline, mockViewport);
  assert.strictEqual(target.kind, 'clip_body');

  const session = createDragSession(600, 60, target, mockViewport.zoom, mockViewport.scrollX, mockViewport.trackHeaderWidth);
  assert.strictEqual(session.mode, 'move_clip');

  // Drag right by 100px (+1.0 second)
  const updated = updateDragSession(session, 700, 60, [], mockViewport.zoom, mockViewport.scrollX, mockViewport.trackHeaderWidth);
  assert.strictEqual(updated.currentTimeSecs, session.startTimeSecs + 1.0);

  const command = resolveDragCommand(updated);
  assert(command !== null);
  assert.strictEqual(command?.type, 'move_clip');
  if (command && command.type === 'move_clip') {
    assert.strictEqual(command.clipId, 'clip-01');
    assert.strictEqual(command.targetTrackId, 'track-01');
    assert.strictEqual(command.newStartSecs, 3.0); // original 2.0 + 1.0 delta
  }
});
