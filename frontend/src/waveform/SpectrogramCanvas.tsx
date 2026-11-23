/**
 * Audio Labs Pro - High-Resolution Spectrogram Canvas
 *
 * Renders time-frequency STFT intensity matrices using calibrated color palettes
 * (Magma, Inferno, Viridis), logarithmic frequency grid lines, and interactive
 * 2D time-frequency bounding box selection with center frequency and Q-factor HUD.
 */

import { useRef, useEffect, useState, useMemo, useCallback } from 'react';
import type { SpectrogramData, SpectralSelectionDto } from '../ipc/types';

export type SpectrogramPalette = 'magma' | 'inferno' | 'viridis';

export interface SpectrogramCanvasProps {
  data: SpectrogramData | null;
  palette?: SpectrogramPalette;
  width?: number;
  height?: number;
  selection?: SpectralSelectionDto | null;
  onSelectionChange?: (selection: SpectralSelectionDto | null) => void;
  showGrid?: boolean;
  interactive?: boolean;
  clipName?: string;
  className?: string;
}

// Colormap stop definitions [normalized_val, r, g, b]
const MAGMA_STOPS: [number, number, number, number][] = [
  [0.0, 0, 0, 4],
  [0.2, 81, 18, 124],
  [0.45, 183, 55, 121],
  [0.7, 252, 137, 97],
  [0.9, 254, 211, 149],
  [1.0, 255, 255, 255],
];

const INFERNO_STOPS: [number, number, number, number][] = [
  [0.0, 0, 0, 4],
  [0.2, 87, 16, 110],
  [0.45, 187, 55, 84],
  [0.7, 249, 142, 9],
  [0.9, 252, 255, 164],
  [1.0, 255, 255, 255],
];

const VIRIDIS_STOPS: [number, number, number, number][] = [
  [0.0, 68, 1, 84],
  [0.25, 59, 82, 139],
  [0.5, 33, 145, 140],
  [0.75, 94, 201, 98],
  [1.0, 253, 231, 37],
];

function generateLut(stops: [number, number, number, number][]): Uint8ClampedArray {
  const lut = new Uint8ClampedArray(256 * 3);
  for (let i = 0; i < 256; i++) {
    const t = i / 255;
    let s0 = stops[0];
    let s1 = stops[stops.length - 1];

    for (let s = 0; s < stops.length - 1; s++) {
      if (t >= stops[s][0] && t <= stops[s + 1][0]) {
        s0 = stops[s];
        s1 = stops[s + 1];
        break;
      }
    }

    const range = s1[0] - s0[0];
    const factor = range > 1e-6 ? (t - s0[0]) / range : 0;

    lut[i * 3 + 0] = Math.round(s0[1] + factor * (s1[1] - s0[1]));
    lut[i * 3 + 1] = Math.round(s0[2] + factor * (s1[2] - s0[2]));
    lut[i * 3 + 2] = Math.round(s0[3] + factor * (s1[3] - s0[3]));
  }
  return lut;
}

const LUTS: Record<SpectrogramPalette, Uint8ClampedArray> = {
  magma: generateLut(MAGMA_STOPS),
  inferno: generateLut(INFERNO_STOPS),
  viridis: generateLut(VIRIDIS_STOPS),
};

// Calibrated frequency grid line steps
const GRID_FREQS_HZ = [50, 100, 250, 500, 1000, 2500, 5000, 10000, 20000];

export function SpectrogramCanvas({
  data,
  palette = 'magma',
  width,
  height = 180,
  selection = null,
  onSelectionChange,
  showGrid = true,
  interactive = true,
  clipName,
  className = '',
}: SpectrogramCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);

  const [activePalette, setActivePalette] = useState<SpectrogramPalette>(palette);
  const [isDragging, setIsDragging] = useState(false);
  const [dragStart, setDragStart] = useState<{ x: number; y: number } | null>(null);
  const [dragCurrent, setDragCurrent] = useState<{ x: number; y: number } | null>(null);
  const [hoverInfo, setHoverInfo] = useState<{ time: number; freq: number; db: number } | null>(null);

  // Sync palette prop
  useEffect(() => {
    setActivePalette(palette);
  }, [palette]);

  // Coordinate transforms
  const durationSec = data?.duration_sec || 10.0;
  const minFreqHz = data?.freq_bins_hz?.[0] || 20.0;
  const maxFreqHz = data?.freq_bins_hz?.[data.freq_bins_hz.length - 1] || 20000.0;

  const yToFreq = useCallback(
    (y: number, canvasHeight: number): number => {
      // Y: 0 is top (maxFreq), canvasHeight is bottom (minFreq)
      const normY = 1.0 - Math.max(0, Math.min(1, y / canvasHeight));
      return minFreqHz * Math.pow(maxFreqHz / minFreqHz, normY);
    },
    [minFreqHz, maxFreqHz]
  );

  const freqToY = useCallback(
    (freq: number, canvasHeight: number): number => {
      const clamped = Math.max(minFreqHz, Math.min(maxFreqHz, freq));
      const logRatio = Math.log(clamped / minFreqHz) / Math.log(maxFreqHz / minFreqHz);
      return (1.0 - logRatio) * canvasHeight;
    },
    [minFreqHz, maxFreqHz]
  );

  const xToTime = useCallback(
    (x: number, canvasWidth: number): number => {
      return (x / canvasWidth) * durationSec;
    },
    [durationSec]
  );

  const timeToX = useCallback(
    (time: number, canvasWidth: number): number => {
      return (time / durationSec) * canvasWidth;
    },
    [durationSec]
  );

  // Render heatmap onto Canvas
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d', { alpha: false });
    if (!ctx) return;

    const w = canvas.width;
    const h = canvas.height;

    if (!data || data.num_frames === 0 || data.num_bins === 0) {
      // Empty background
      ctx.fillStyle = '#090d16';
      ctx.fillRect(0, 0, w, h);
      ctx.fillStyle = '#475569';
      ctx.font = '12px "JetBrains Mono", monospace';
      ctx.textAlign = 'center';
      ctx.fillText('No spectrogram data', w / 2, h / 2);
      return;
    }

    const { num_frames, num_bins, matrix } = data;
    const lut = LUTS[activePalette] || LUTS.magma;

    // Render into an offscreen ImageData buffer at spectrogram resolution, then draw scaled
    const offscreen = document.createElement('canvas');
    offscreen.width = num_frames;
    offscreen.height = num_bins;
    const offCtx = offscreen.getContext('2d');
    if (!offCtx) return;

    const imgData = offCtx.createImageData(num_frames, num_bins);
    const pixels = imgData.data;

    // Heatmap: Row 0 is highest freq (top), Row num_bins-1 is lowest freq (bottom)
    for (let f = 0; f < num_frames; f++) {
      for (let b = 0; b < num_bins; b++) {
        // matrix: frame f and bin b (where bin 0 is lowest freq)
        // For visual display, row 0 is top (bin num_bins - 1 - b)
        const binIndex = num_bins - 1 - b;
        const intensity = matrix[f * num_bins + binIndex] || 0.0;
        const lutIdx = Math.max(0, Math.min(255, Math.floor(intensity * 255))) * 3;

        const pixelIdx = (b * num_frames + f) * 4;
        pixels[pixelIdx + 0] = lut[lutIdx + 0];
        pixels[pixelIdx + 1] = lut[lutIdx + 1];
        pixels[pixelIdx + 2] = lut[lutIdx + 2];
        pixels[pixelIdx + 3] = 255;
      }
    }

    offCtx.putImageData(imgData, 0, 0);

    // Bilinear draw to main canvas
    ctx.imageSmoothingEnabled = true;
    ctx.imageSmoothingQuality = 'high';
    ctx.drawImage(offscreen, 0, 0, w, h);

    // Grid overlays
    if (showGrid) {
      ctx.save();
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.12)';
      ctx.lineWidth = 1;
      ctx.setLineDash([3, 3]);

      ctx.font = '9px "JetBrains Mono", monospace';
      ctx.fillStyle = 'rgba(255, 255, 255, 0.45)';
      ctx.textAlign = 'right';

      for (const f of GRID_FREQS_HZ) {
        if (f >= minFreqHz && f <= maxFreqHz) {
          const y = Math.round(freqToY(f, h)) + 0.5;
          ctx.beginPath();
          ctx.moveTo(0, y);
          ctx.lineTo(w, y);
          ctx.stroke();

          const label = f >= 1000 ? `${f / 1000}k` : `${f}`;
          ctx.fillText(label, w - 6, y - 2);
        }
      }
      ctx.restore();
    }

    // Render active spectral selection box
    const currentSelection = isDragging && dragStart && dragCurrent
      ? {
          t_start: Math.min(xToTime(dragStart.x, w), xToTime(dragCurrent.x, w)),
          t_end: Math.max(xToTime(dragStart.x, w), xToTime(dragCurrent.x, w)),
          f_min: Math.min(yToFreq(dragStart.y, h), yToFreq(dragCurrent.y, h)),
          f_max: Math.max(yToFreq(dragStart.y, h), yToFreq(dragCurrent.y, h)),
        }
      : selection
      ? {
          t_start: selection.start_secs,
          t_end: selection.end_secs,
          f_min: selection.f_min_hz,
          f_max: selection.f_max_hz,
        }
      : null;

    if (currentSelection) {
      const x1 = timeToX(currentSelection.t_start, w);
      const x2 = timeToX(currentSelection.t_end, w);
      const y1 = freqToY(currentSelection.f_max, h);
      const y2 = freqToY(currentSelection.f_min, h);

      const selW = Math.max(2, x2 - x1);
      const selH = Math.max(2, y2 - y1);

      ctx.save();
      // Translucent cyan overlay
      ctx.fillStyle = 'rgba(6, 182, 212, 0.2)';
      ctx.fillRect(x1, y1, selW, selH);

      // Neon cyan boundary with glowing dash
      ctx.strokeStyle = '#22d3ee';
      ctx.lineWidth = 1.5;
      ctx.setLineDash([4, 2]);
      ctx.strokeRect(x1, y1, selW, selH);

      // Corner grab handles
      const handleSize = 6;
      ctx.fillStyle = '#ffffff';
      ctx.strokeStyle = '#0891b2';
      ctx.lineWidth = 1;
      ctx.setLineDash([]);

      const corners = [
        [x1, y1],
        [x1 + selW, y1],
        [x1, y1 + selH],
        [x1 + selW, y1 + selH],
      ];

      for (const [cx, cy] of corners) {
        ctx.fillRect(cx - handleSize / 2, cy - handleSize / 2, handleSize, handleSize);
        ctx.strokeRect(cx - handleSize / 2, cy - handleSize / 2, handleSize, handleSize);
      }

      ctx.restore();
    }
  }, [
    data,
    activePalette,
    showGrid,
    selection,
    isDragging,
    dragStart,
    dragCurrent,
    minFreqHz,
    maxFreqHz,
    durationSec,
    freqToY,
    timeToX,
    xToTime,
    yToFreq,
  ]);

  // Handle Canvas Resizing
  useEffect(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container) return;

    const resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const { width: containerWidth } = entry.contentRect;
        const targetWidth = Math.floor(width || containerWidth || 600);
        if (canvas.width !== targetWidth || canvas.height !== height) {
          canvas.width = targetWidth;
          canvas.height = height;
        }
      }
    });

    resizeObserver.observe(container);
    return () => resizeObserver.disconnect();
  }, [width, height]);

  // Mouse / Drag handlers for 2D Spectral Selection Box
  const handleMouseDown = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!interactive) return;
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    setIsDragging(true);
    setDragStart({ x, y });
    setDragCurrent({ x, y });
  };

  const handleMouseMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = Math.max(0, Math.min(canvas.width, e.clientX - rect.left));
    const y = Math.max(0, Math.min(canvas.height, e.clientY - rect.top));

    if (isDragging) {
      setDragCurrent({ x, y });
    }

    // Update hover inspector HUD
    const time = xToTime(x, canvas.width);
    const freq = yToFreq(y, canvas.height);
    let db = -100.0;
    if (data && data.num_frames > 0 && data.num_bins > 0) {
      const fIdx = Math.min(data.num_frames - 1, Math.floor((time / durationSec) * data.num_frames));
      const bIdx = Math.min(
        data.num_bins - 1,
        Math.floor((Math.log(freq / minFreqHz) / Math.log(maxFreqHz / minFreqHz)) * data.num_bins)
      );
      if (fIdx >= 0 && bIdx >= 0) {
        const intensity = data.matrix[fIdx * data.num_bins + bIdx] || 0.0;
        db = data.min_db + intensity * (data.max_db - data.min_db);
      }
    }
    setHoverInfo({ time, freq, db });
  };

  const handleMouseUp = () => {
    if (!isDragging) return;
    setIsDragging(false);

    if (dragStart && dragCurrent && canvasRef.current) {
      const w = canvasRef.current.width;
      const h = canvasRef.current.height;

      const t1 = Math.min(xToTime(dragStart.x, w), xToTime(dragCurrent.x, w));
      const t2 = Math.max(xToTime(dragStart.x, w), xToTime(dragCurrent.x, w));
      const f1 = Math.min(yToFreq(dragStart.y, h), yToFreq(dragCurrent.y, h));
      const f2 = Math.max(yToFreq(dragStart.y, h), yToFreq(dragCurrent.y, h));

      // Minimum box size threshold (5px in both axes)
      if (Math.abs(dragCurrent.x - dragStart.x) > 5 && Math.abs(dragCurrent.y - dragStart.y) > 5) {
        const center_freq_hz = Math.sqrt(f1 * f2);
        const bandwidth_hz = f2 - f1;
        const q_factor = bandwidth_hz > 1e-2 ? center_freq_hz / bandwidth_hz : 0;

        const newSelection: SpectralSelectionDto = {
          track_id: selection?.track_id || '',
          start_secs: Math.round(t1 * 1000) / 1000,
          end_secs: Math.round(t2 * 1000) / 1000,
          f_min_hz: Math.round(f1 * 10) / 10,
          f_max_hz: Math.round(f2 * 10) / 10,
          center_freq_hz: Math.round(center_freq_hz * 10) / 10,
          bandwidth_hz: Math.round(bandwidth_hz * 10) / 10,
          q_factor: Math.round(q_factor * 100) / 100,
        };
        onSelectionChange?.(newSelection);
      }
    }

    setDragStart(null);
    setDragCurrent(null);
  };

  const handleClearSelection = (e: React.MouseEvent) => {
    e.stopPropagation();
    onSelectionChange?.(null);
  };

  // Computed HUD values
  const activeBox = useMemo(() => {
    if (isDragging && dragStart && dragCurrent && canvasRef.current) {
      const w = canvasRef.current.width;
      const h = canvasRef.current.height;
      const t1 = Math.min(xToTime(dragStart.x, w), xToTime(dragCurrent.x, w));
      const t2 = Math.max(xToTime(dragStart.x, w), xToTime(dragCurrent.x, w));
      const f1 = Math.min(yToFreq(dragStart.y, h), yToFreq(dragCurrent.y, h));
      const f2 = Math.max(yToFreq(dragStart.y, h), yToFreq(dragCurrent.y, h));
      const fc = Math.sqrt(f1 * f2);
      const bw = f2 - f1;
      const q = bw > 1e-2 ? fc / bw : 0;
      return { t1, t2, f1, f2, fc, bw, q };
    }
    if (selection) {
      return {
        t1: selection.start_secs,
        t2: selection.end_secs,
        f1: selection.f_min_hz,
        f2: selection.f_max_hz,
        fc: selection.center_freq_hz,
        bw: selection.bandwidth_hz,
        q: selection.q_factor,
      };
    }
    return null;
  }, [isDragging, dragStart, dragCurrent, selection, xToTime, yToFreq]);

  return (
    <div
      ref={containerRef}
      className={`relative w-full rounded-md border border-slate-800 bg-slate-950/80 overflow-hidden select-none ${className}`}
      style={{ height }}
    >
      {/* Top Toolbar: Palette selector, HUD badge, Clear */}
      <div className="absolute top-2 left-2 right-2 flex items-center justify-between z-10 pointer-events-none">
        <div className="flex items-center gap-2 pointer-events-auto">
          {clipName && (
            <span className="text-[10px] font-mono font-medium text-slate-300 bg-slate-900/80 border border-slate-700/60 px-2 py-0.5 rounded shadow-sm">
              {clipName}
            </span>
          )}
          {/* Palette switcher */}
          <div className="flex items-center bg-slate-900/90 border border-slate-800 rounded p-0.5 text-[10px] font-mono shadow">
            {(['magma', 'inferno', 'viridis'] as SpectrogramPalette[]).map((pal) => (
              <button
                key={pal}
                type="button"
                onClick={() => setActivePalette(pal)}
                className={`px-1.5 py-0.5 rounded capitalize transition-colors ${
                  activePalette === pal
                    ? 'bg-cyan-500/20 text-cyan-300 font-semibold border border-cyan-500/40'
                    : 'text-slate-400 hover:text-slate-200'
                }`}
              >
                {pal}
              </button>
            ))}
          </div>
        </div>

        {/* 2D Spectral Selection HUD */}
        {activeBox && (
          <div className="pointer-events-auto flex items-center gap-2.5 bg-slate-900/95 border border-cyan-500/50 rounded px-2.5 py-1 text-[11px] font-mono text-cyan-300 shadow-lg backdrop-blur-sm">
            <span className="text-slate-400">Δt:</span>
            <span className="font-semibold text-white">
              {(activeBox.t2 - activeBox.t1).toFixed(2)}s
            </span>
            <span className="text-slate-500">|</span>
            <span className="text-slate-400">Range:</span>
            <span className="font-semibold text-white">
              {activeBox.f1 >= 1000 ? `${(activeBox.f1 / 1000).toFixed(1)}k` : Math.round(activeBox.f1)}
              {' → '}
              {activeBox.f2 >= 1000 ? `${(activeBox.f2 / 1000).toFixed(1)}k` : Math.round(activeBox.f2)}Hz
            </span>
            <span className="text-slate-500">|</span>
            <span className="text-slate-400">fc:</span>
            <span className="font-semibold text-amber-300">
              {activeBox.fc >= 1000 ? `${(activeBox.fc / 1000).toFixed(2)} kHz` : `${Math.round(activeBox.fc)} Hz`}
            </span>
            <span className="text-slate-500">|</span>
            <span className="text-slate-400">Q:</span>
            <span className="font-semibold text-emerald-300">{activeBox.q.toFixed(2)}</span>
            <button
              type="button"
              onClick={handleClearSelection}
              className="ml-1 text-slate-400 hover:text-rose-400 transition-colors"
              title="Clear spectral selection"
            >
              ✕
            </button>
          </div>
        )}
      </div>

      {/* Main Spectrogram Canvas */}
      <canvas
        ref={canvasRef}
        height={height}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={() => {
          if (isDragging) handleMouseUp();
          setHoverInfo(null);
        }}
        className={`w-full h-full block ${interactive ? 'cursor-crosshair' : 'cursor-default'}`}
      />

      {/* Bottom Status / Hover Inspector */}
      {hoverInfo && (
        <div className="absolute bottom-1.5 left-2 pointer-events-none bg-slate-950/80 border border-slate-800/80 px-2 py-0.5 rounded text-[10px] font-mono text-slate-400 flex items-center gap-2">
          <span>t: {hoverInfo.time.toFixed(3)}s</span>
          <span className="text-slate-600">·</span>
          <span>f: {hoverInfo.freq >= 1000 ? `${(hoverInfo.freq / 1000).toFixed(2)}kHz` : `${Math.round(hoverInfo.freq)}Hz`}</span>
          <span className="text-slate-600">·</span>
          <span>dB: {hoverInfo.db.toFixed(1)} dBFS</span>
        </div>
      )}
    </div>
  );
}
