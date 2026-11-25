/**
 * Audio Labs Pro - High Performance Waveform Canvas
 *
 * Renders min/max audio peaks from a multi-resolution peak pyramid.
 * Supports dynamic decimation, level-of-detail selection by zoom,
 * playhead scrubbing, loop region overlays, and 1-hour autoscroll.
 */

import { useRef, useEffect, useCallback, useState, useMemo } from 'react';
import type { WaveformPeaksDto } from '../ipc/types';
import { readLinearCanvasTokens, useLinearCanvasTheme } from '../utils/linearCanvasTheme';

export interface WaveformCanvasProps {
  peaks: WaveformPeaksDto;
  currentTime: number;
  duration: number;
  isPlaying: boolean;
  loopEnabled?: boolean;
  loopStart?: number;
  loopEnd?: number;
  selectionStart?: number;
  selectionEnd?: number;
  onSeek?: (timeSecs: number) => void;
  onSelectRange?: (startSecs: number, endSecs: number) => void;
  autoscroll?: boolean;
  height?: number;
  accentColor?: string;
}

export function WaveformCanvas({
  peaks,
  currentTime,
  duration,
  isPlaying,
  loopEnabled = false,
  loopStart = 0,
  loopEnd = 0,
  selectionStart = 0,
  selectionEnd = 0,
  onSeek,
  onSelectRange,
  autoscroll = true,
  height = 200,
  accentColor,
}: WaveformCanvasProps) {
  const themeTick = useLinearCanvasTheme();
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);

  // Viewport scroll window (in seconds)
  const [viewStart, setViewStart] = useState<number>(0);
  const [viewDuration, setViewDuration] = useState<number>(duration);

  // Drag interaction state
  const [isDraggingPlayhead, setIsDraggingPlayhead] = useState<boolean>(false);
  const [isSelecting, setIsSelecting] = useState<boolean>(false);
  const [dragAnchorTime, setDragAnchorTime] = useState<number>(0);

  // Performance telemetry for M0/M4 benchmark
  const [fps, setFps] = useState<number>(60);
  const frameTimesRef = useRef<number[]>([]);

  // Update viewDuration when duration changes (e.g. switching between demo & 1-hour)
  useEffect(() => {
    setViewDuration(Math.min(viewDuration, duration));
  }, [duration, viewDuration]);

  // Autoscroll playhead into view when playing
  useEffect(() => {
    if (!isPlaying || !autoscroll) return;

    // Scroll if playhead is past 85% of visible window or behind viewStart
    if (currentTime > viewStart + viewDuration * 0.85 || currentTime < viewStart) {
      const nextStart = Math.max(0, Math.min(currentTime - viewDuration * 0.15, duration - viewDuration));
      setViewStart(nextStart);
    }
  }, [currentTime, isPlaying, autoscroll, viewStart, viewDuration, duration]);

  // Select pyramid level based on samples per pixel
  const activeLevel = useMemo(() => {
    if (!canvasRef.current || !peaks.levels || peaks.levels.length === 0) {
      return peaks.levels[0];
    }
    const canvasWidth = canvasRef.current.clientWidth || 800;
    const samplesPerPixel = (viewDuration * peaks.sample_rate) / canvasWidth;

    let chosen = peaks.levels[0];
    for (const lvl of peaks.levels) {
      if (lvl.bucket_samples <= samplesPerPixel) {
        chosen = lvl;
      } else {
        break;
      }
    }
    return chosen;
  }, [peaks, viewDuration]);

  // Render routine
  const render = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const width = canvas.clientWidth;
    const canvasHeight = canvas.clientHeight;

    if (canvas.width !== width * dpr || canvas.height !== canvasHeight * dpr) {
      canvas.width = width * dpr;
      canvas.height = canvasHeight * dpr;
    }

    ctx.save();
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, width, canvasHeight);

    const rulerHeight = 22;
    const waveTop = rulerHeight;
    const waveHeight = canvasHeight - rulerHeight;
    const waveMidY = waveTop + waveHeight / 2;
    const halfWave = waveHeight / 2 - 4;
    const tokens = readLinearCanvasTokens();

    // 1. Background Grid & Axis (--bg-surface-2)
    ctx.fillStyle = tokens.bgSurface2;
    ctx.fillRect(0, 0, width, canvasHeight);

    // Center zero axis
    ctx.strokeStyle = tokens.border;
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, waveMidY);
    ctx.lineTo(width, waveMidY);
    ctx.stroke();

    // 2. Selection Region
    if (selectionEnd > selectionStart) {
      const selX1 = ((selectionStart - viewStart) / viewDuration) * width;
      const selX2 = ((selectionEnd - viewStart) / viewDuration) * width;
      ctx.fillStyle = tokens.tintSky;
      ctx.fillRect(selX1, waveTop, selX2 - selX1, waveHeight);
      ctx.strokeStyle = tokens.accentSky;
      ctx.strokeRect(selX1, waveTop, selX2 - selX1, waveHeight);
    }

    // 3. Loop Region Overlay
    if (loopEnabled && loopEnd > loopStart) {
      const loopX1 = ((loopStart - viewStart) / viewDuration) * width;
      const loopX2 = ((loopEnd - viewStart) / viewDuration) * width;
      ctx.fillStyle = tokens.tintPurple;
      ctx.fillRect(loopX1, waveTop, loopX2 - loopX1, waveHeight);

      // Loop boundary markers
      ctx.strokeStyle = tokens.accentPurple;
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.moveTo(loopX1, waveTop);
      ctx.lineTo(loopX1, waveTop + waveHeight);
      ctx.moveTo(loopX2, waveTop);
      ctx.lineTo(loopX2, waveTop + waveHeight);
      ctx.stroke();
    }

    // 4. Flat Waveform Peak Bars (Sky on --bg-surface-2, zero gradients)
    if (activeLevel && activeLevel.pairs.length > 0) {
      const bucketSamples = activeLevel.bucket_samples;
      const totalPairs = activeLevel.pairs.length;

      ctx.fillStyle = accentColor || tokens.accentSky;

      const visibleStartFrame = viewStart * peaks.sample_rate;
      const visibleEndFrame = (viewStart + viewDuration) * peaks.sample_rate;
      const framesPerPixel = (visibleEndFrame - visibleStartFrame) / width;

      for (let px = 0; px < width; px++) {
        const frameStart = visibleStartFrame + px * framesPerPixel;
        const frameEnd = frameStart + framesPerPixel;

        const firstBucket = Math.floor(frameStart / bucketSamples);
        const lastBucket = Math.min(totalPairs - 1, Math.floor(frameEnd / bucketSamples));

        if (firstBucket >= totalPairs || lastBucket < 0) continue;

        let minVal = 32767;
        let maxVal = -32768;

        for (let b = Math.max(0, firstBucket); b <= lastBucket; b++) {
          const [lo, hi] = activeLevel.pairs[b];
          if (lo < minVal) minVal = lo;
          if (hi > maxVal) maxVal = hi;
        }

        if (minVal > maxVal) continue;

        const normMin = minVal / 32768;
        const normMax = maxVal / 32768;

        const yTop = waveMidY - normMax * halfWave;
        const yBottom = waveMidY - normMin * halfWave;
        const barH = Math.max(1, yBottom - yTop);

        ctx.fillRect(px, yTop, 1, barH);
      }
    }

    // 5. Dotted Timecode Ruler at Top
    ctx.fillStyle = tokens.bgSurface;
    ctx.fillRect(0, 0, width, rulerHeight);
    ctx.strokeStyle = tokens.border;
    ctx.beginPath();
    ctx.moveTo(0, rulerHeight);
    ctx.lineTo(width, rulerHeight);
    ctx.stroke();

    // Ruler tick marks (dotted grid)
    ctx.fillStyle = tokens.textMuted;
    ctx.font = '10px ui-monospace, monospace';
    const tickIntervalSecs = viewDuration > 600 ? 120 : viewDuration > 60 ? 10 : 2;
    const firstTick = Math.ceil(viewStart / tickIntervalSecs) * tickIntervalSecs;

    for (let t = firstTick; t < viewStart + viewDuration; t += tickIntervalSecs) {
      const x = ((t - viewStart) / viewDuration) * width;
      ctx.save();
      ctx.strokeStyle = tokens.borderStrong;
      ctx.setLineDash([1, 2]);
      ctx.beginPath();
      ctx.moveTo(x, rulerHeight - 6);
      ctx.lineTo(x, rulerHeight);
      ctx.stroke();
      ctx.restore();

      const mins = Math.floor(t / 60);
      const secs = Math.floor(t % 60);
      const label = `${mins}:${secs.toString().padStart(2, '0')}`;
      ctx.fillText(label, x + 3, rulerHeight - 8);
    }

    // 6. Red Playhead Line & Cursor Flag (zero glow)
    const playheadX = ((currentTime - viewStart) / viewDuration) * width;
    if (playheadX >= 0 && playheadX <= width) {
      ctx.strokeStyle = tokens.accentRed;
      ctx.lineWidth = 1.5;

      ctx.beginPath();
      ctx.moveTo(playheadX, 0);
      ctx.lineTo(playheadX, canvasHeight);
      ctx.stroke();

      ctx.fillStyle = tokens.accentRed;
      ctx.beginPath();
      ctx.moveTo(playheadX - 5, 0);
      ctx.lineTo(playheadX + 5, 0);
      ctx.lineTo(playheadX, 9);
      ctx.closePath();
      ctx.fill();
    }

    ctx.restore();

    // Track FPS metrics during autoscroll
    if (isPlaying) {
      const now = performance.now();
      frameTimesRef.current.push(now);
      if (frameTimesRef.current.length > 30) {
        frameTimesRef.current.shift();
        const delta = (now - frameTimesRef.current[0]) / (frameTimesRef.current.length - 1);
        setFps(Math.round(1000 / delta));
      }
    }
  }, [
    activeLevel,
    currentTime,
    duration,
    isPlaying,
    loopEnabled,
    loopStart,
    loopEnd,
    selectionStart,
    selectionEnd,
    viewStart,
    viewDuration,
    accentColor,
    peaks.sample_rate,
    themeTick,
  ]);

  // Request render only on relevant updates (zero-idle-draw compliant)
  useEffect(() => {
    let animId: number;
    if (isPlaying) {
      const loop = () => {
        render();
        animId = requestAnimationFrame(loop);
      };
      animId = requestAnimationFrame(loop);
    } else {
      render();
    }

    return () => {
      if (animId) cancelAnimationFrame(animId);
    };
  }, [isPlaying, render]);

  // Mouse scrubbing & interaction handlers
  const handleMouseDown = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const clickRatio = Math.max(0, Math.min(1, x / canvas.clientWidth));
    const clickTime = viewStart + clickRatio * viewDuration;

    if (e.shiftKey) {
      // Range selection drag
      setIsSelecting(true);
      setDragAnchorTime(clickTime);
      onSelectRange?.(clickTime, clickTime);
    } else {
      // Seek playhead
      setIsDraggingPlayhead(true);
      onSeek?.(clickTime);
    }
  };

  const handleMouseMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const moveRatio = Math.max(0, Math.min(1, x / canvas.clientWidth));
    const moveTime = viewStart + moveRatio * viewDuration;

    if (isDraggingPlayhead) {
      onSeek?.(moveTime);
    } else if (isSelecting) {
      const start = Math.min(dragAnchorTime, moveTime);
      const end = Math.max(dragAnchorTime, moveTime);
      onSelectRange?.(start, end);
    }
  };

  const handleMouseUp = () => {
    setIsDraggingPlayhead(false);
    setIsSelecting(false);
  };

  // Zoom at cursor with mouse wheel
  const handleWheel = (e: React.WheelEvent<HTMLCanvasElement>) => {
    e.preventDefault();
    const zoomFactor = e.deltaY < 0 ? 0.8 : 1.25;
    const canvas = canvasRef.current;
    if (!canvas) return;

    const rect = canvas.getBoundingClientRect();
    const cursorRatio = (e.clientX - rect.left) / canvas.clientWidth;
    const cursorTime = viewStart + cursorRatio * viewDuration;

    const nextDuration = Math.max(2.0, Math.min(duration, viewDuration * zoomFactor));
    const nextStart = Math.max(0, Math.min(cursorTime - cursorRatio * nextDuration, duration - nextDuration));

    setViewDuration(nextDuration);
    setViewStart(nextStart);
  };

  return (
    <div ref={containerRef} className="relative w-full flex flex-col select-none bg-[var(--bg-surface-2)] rounded-lg border border-[var(--border)] overflow-hidden">
      {/* Waveform Canvas */}
      <canvas
        ref={canvasRef}
        height={height}
        className="w-full cursor-crosshair block"
        style={{ height: `${height}px` }}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseUp}
        onWheel={handleWheel}
      />

      {/* Viewport Telemetry Overlay */}
      <div className="absolute top-2 right-3 flex items-center gap-2 pointer-events-none text-[11px] font-mono">
        <span className="px-1.5 py-0.5 rounded bg-[var(--bg-surface)] text-[var(--text-muted)] border border-[var(--border)] tabular-nums">
          Level: {activeLevel?.bucket_samples} spl ({peaks.levels.indexOf(activeLevel)})
        </span>
        {isPlaying && (
          <span
            className={`px-1.5 py-0.5 rounded font-medium border tabular-nums ${
              fps >= 55
                ? 'bg-[var(--bg-surface)] text-[var(--text-muted)] border-[var(--border-strong)]'
                : 'bg-[var(--tint-amber)] text-[var(--accent-amber)] border-[var(--accent-amber)]'
            }`}
          >
            {fps} FPS
          </span>
        )}
      </div>
    </div>
  );
}
