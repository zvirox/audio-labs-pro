/**
 * Audio Labs Pro - Multitrack Canvas 2D Timeline Component
 *
 * High-performance canvas-based timeline editor supporting:
 * - Multi-track lanes with mute, solo, arm, and gain/pan controls
 * - Audio clips with waveform previews, fade-in/out, and envelope curves
 * - Timecode ruler with sub-second ticks, loop boundaries, and markers
 * - Interactive dragging (move, trim, fades, loop adjustment, playhead scrubbing)
 * - Magnetic snapping to markers, playhead, and adjacent clip edges
 */

import React, { useRef, useEffect, useState, useCallback } from 'react';
import type {
  TimelineDto,
  ViewportConfig,
  HitTarget,
  DragState,
  TimelineCommandPayload,
} from './types';
import {
  timeToPixels,
  pixelsToTime,
  formatTimecode,
  hitTest,
} from './timelineMath';
import {
  createDragSession,
  updateDragSession,
  resolveDragCommand,
} from './dragLogic';

export interface MultitrackCanvasProps {
  timeline: TimelineDto;
  currentTimeSecs: number;
  isPlaying: boolean;
  onSeek: (secs: number) => void;
  onSelectClip: (clipId: string | null) => void;
  onDispatchCommand: (cmd: TimelineCommandPayload) => void;
  zoom: number;
  onZoomChange?: (newZoom: number) => void;
  snapEnabled?: boolean;
  rippleEnabled?: boolean;
}

const RULER_HEIGHT = 32;
const TRACK_HEIGHT = 88;
const TRACK_HEADER_WIDTH = 180;

export function MultitrackCanvas({
  timeline,
  currentTimeSecs,
  isPlaying,
  onSeek,
  onSelectClip,
  onDispatchCommand,
  zoom,
  onZoomChange,
  snapEnabled = true,
}: MultitrackCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);

  const [scrollX, setScrollX] = useState<number>(0);
  const [scrollY, setScrollY] = useState<number>(0);
  const [dragState, setDragState] = useState<DragState | null>(null);
  const [hoverTarget, setHoverTarget] = useState<HitTarget | null>(null);
  const [canvasSize, setCanvasSize] = useState<{ width: number; height: number }>({
    width: 1000,
    height: 500,
  });

  // Track container resize
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (entry) {
        setCanvasSize({
          width: Math.floor(entry.contentRect.width),
          height: Math.floor(entry.contentRect.height),
        });
      }
    });
    observer.observe(container);
    return () => observer.disconnect();
  }, []);

  const viewport: ViewportConfig = {
    zoom,
    scrollX,
    scrollY,
    trackHeight: TRACK_HEIGHT,
    rulerHeight: RULER_HEIGHT,
    trackHeaderWidth: TRACK_HEADER_WIDTH,
    width: canvasSize.width,
    height: canvasSize.height,
  };

  // Collect snap candidates (markers, playhead, clip boundaries)
  const getSnapCandidates = useCallback((): number[] => {
    if (!snapEnabled) return [];
    const candidates = [0, currentTimeSecs];
    for (const marker of timeline.markers) {
      candidates.push(marker.time_secs);
    }
    if (timeline.loop_range) {
      candidates.push(timeline.loop_range.start_secs, timeline.loop_range.end_secs);
    }
    for (const track of timeline.tracks) {
      for (const clip of track.clips) {
        candidates.push(clip.timeline_start_secs);
        candidates.push(clip.timeline_start_secs + clip.duration_secs);
      }
    }
    return candidates;
  }, [timeline, currentTimeSecs, snapEnabled]);

  // Canvas drawing loop
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    canvas.width = canvasSize.width * dpr;
    canvas.height = canvasSize.height * dpr;
    ctx.scale(dpr, dpr);

    const { width, height } = canvasSize;

    // 1. Background
    ctx.fillStyle = '#090d16';
    ctx.fillRect(0, 0, width, height);

    // 2. Track Lanes Background & Horizontal Grid
    timeline.tracks.forEach((track, index) => {
      const trackTop = RULER_HEIGHT + (index * TRACK_HEIGHT) - scrollY;
      if (trackTop + TRACK_HEIGHT < RULER_HEIGHT || trackTop > height) return;

      // Alternating subtle lane shade
      ctx.fillStyle = index % 2 === 0 ? '#0d1322' : '#0a0f1c';
      ctx.fillRect(TRACK_HEADER_WIDTH, trackTop, width - TRACK_HEADER_WIDTH, TRACK_HEIGHT);

      // Track bottom border
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.07)';
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(0, trackTop + TRACK_HEIGHT);
      ctx.lineTo(width, trackTop + TRACK_HEIGHT);
      ctx.stroke();

      // Track Header Column (180px)
      ctx.fillStyle = '#0f172a';
      ctx.fillRect(0, trackTop, TRACK_HEADER_WIDTH, TRACK_HEIGHT);
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.1)';
      ctx.strokeRect(0, trackTop, TRACK_HEADER_WIDTH, TRACK_HEIGHT);

      // Track Title & Index
      ctx.fillStyle = '#f8fafc';
      ctx.font = 'bold 11px system-ui, sans-serif';
      ctx.fillText(`${index + 1}. ${track.name}`, 12, trackTop + 20);

      // Mute / Solo / Arm Status Pills
      const drawPill = (label: string, active: boolean, color: string, x: number) => {
        ctx.fillStyle = active ? color : 'rgba(255, 255, 255, 0.08)';
        ctx.strokeStyle = active ? color : 'rgba(255, 255, 255, 0.15)';
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.roundRect(x, trackTop + 32, 22, 16, 3);
        ctx.fill();
        ctx.stroke();

        ctx.fillStyle = active ? '#000000' : '#94a3b8';
        ctx.font = 'bold 9px system-ui, sans-serif';
        ctx.textAlign = 'center';
        ctx.fillText(label, x + 11, trackTop + 44);
        ctx.textAlign = 'left';
      };

      drawPill('M', track.muted, '#f43f5e', 10);
      drawPill('S', track.solo, '#eab308', 34);
      drawPill('R', track.armed, '#ef4444', 58);

      // View Mode Selector Pills (Waveform, Spectrogram, Split)
      const drawModePill = (label: string, active: boolean, color: string, x: number, w: number) => {
        ctx.fillStyle = active ? `${color}33` : 'rgba(255, 255, 255, 0.05)';
        ctx.strokeStyle = active ? color : 'rgba(255, 255, 255, 0.12)';
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.roundRect(x, trackTop + 32, w, 16, 3);
        ctx.fill();
        ctx.stroke();

        ctx.fillStyle = active ? color : '#64748b';
        ctx.font = 'bold 8.5px system-ui, sans-serif';
        ctx.textAlign = 'center';
        ctx.fillText(label, x + w / 2, trackTop + 43.5);
        ctx.textAlign = 'left';
      };

      const currentMode = track.view_mode || 'waveform';
      drawModePill('W', currentMode === 'waveform', '#38bdf8', 86, 18);
      drawModePill('S', currentMode === 'spectrogram', '#f97316', 108, 18);
      drawModePill('Dual', currentMode === 'split', '#a855f7', 130, 38);

      // Fader & Pan readouts
      ctx.fillStyle = '#64748b';
      ctx.font = '10px monospace';
      ctx.fillText(`Vol: ${(track.gain >= 1 ? '+' : '')}${(20 * Math.log10(Math.max(0.001, track.gain))).toFixed(1)} dB`, 10, trackTop + 68);
      ctx.fillText(`Pan: ${track.pan === 0 ? 'C' : track.pan < 0 ? `L${Math.round(-track.pan * 100)}` : `R${Math.round(track.pan * 100)}`}`, 96, trackTop + 68);
    });

    // 3. Vertical Grid Lines (Seconds & Bars)
    const startTimeSecs = pixelsToTime(TRACK_HEADER_WIDTH, zoom, scrollX, TRACK_HEADER_WIDTH);
    const endTimeSecs = pixelsToTime(width, zoom, scrollX, TRACK_HEADER_WIDTH);
    const gridStepSecs = zoom > 150 ? 0.5 : zoom > 60 ? 1.0 : zoom > 20 ? 5.0 : 10.0;

    const firstGridSec = Math.floor(startTimeSecs / gridStepSecs) * gridStepSecs;
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.04)';
    ctx.lineWidth = 1;

    for (let t = firstGridSec; t <= endTimeSecs; t += gridStepSecs) {
      const x = timeToPixels(t, zoom, scrollX, TRACK_HEADER_WIDTH);
      ctx.beginPath();
      ctx.moveTo(x, RULER_HEIGHT);
      ctx.lineTo(x, height);
      ctx.stroke();
    }

    // 4. Clips & Waveforms
    timeline.tracks.forEach((track, trackIndex) => {
      const trackTop = RULER_HEIGHT + (trackIndex * TRACK_HEIGHT) - scrollY;
      if (trackTop + TRACK_HEIGHT < RULER_HEIGHT || trackTop > height) return;

      track.clips.forEach((clip) => {
        let clipStartX = timeToPixels(clip.timeline_start_secs, zoom, scrollX, TRACK_HEADER_WIDTH);
        let clipWidth = Math.max(4, clip.duration_secs * zoom);

        // Apply visual drag ghost offset if currently dragging this clip
        if (dragState && dragState.mode === 'move_clip' && dragState.target.clip?.id === clip.id) {
          const deltaSecs = dragState.snappedTimeSecs - dragState.startTimeSecs;
          clipStartX = timeToPixels(clip.timeline_start_secs + deltaSecs, zoom, scrollX, TRACK_HEADER_WIDTH);
        } else if (dragState && dragState.mode === 'trim_clip_left' && dragState.target.clip?.id === clip.id) {
          const deltaSecs = dragState.snappedTimeSecs - dragState.startTimeSecs;
          const newStart = Math.max(0, clip.timeline_start_secs + deltaSecs);
          const newDur = Math.max(0.05, clip.duration_secs - (newStart - clip.timeline_start_secs));
          clipStartX = timeToPixels(newStart, zoom, scrollX, TRACK_HEADER_WIDTH);
          clipWidth = Math.max(4, newDur * zoom);
        } else if (dragState && dragState.mode === 'trim_clip_right' && dragState.target.clip?.id === clip.id) {
          const deltaSecs = dragState.snappedTimeSecs - dragState.startTimeSecs;
          clipWidth = Math.max(4, (clip.duration_secs + deltaSecs) * zoom);
        }

        const isSelected = timeline.selected_clip_id === clip.id;
        const clipY = trackTop + 6;
        const clipH = TRACK_HEIGHT - 12;
        const viewMode = track.view_mode || 'waveform';

        // Clip Container Rounded Rectangle
        ctx.save();
        ctx.beginPath();
        ctx.roundRect(clipStartX, clipY, clipWidth, clipH, 6);
        ctx.clip();

        // Clip Background Fill
        const grad = ctx.createLinearGradient(clipStartX, clipY, clipStartX, clipY + clipH);
        grad.addColorStop(0, isSelected ? 'rgba(6, 182, 212, 0.28)' : 'rgba(30, 41, 59, 0.7)');
        grad.addColorStop(1, isSelected ? 'rgba(14, 116, 144, 0.35)' : 'rgba(15, 23, 42, 0.85)');
        ctx.fillStyle = grad;
        ctx.fillRect(clipStartX, clipY, clipWidth, clipH);

        // Multi-view renderer: Waveform, Spectrogram, or Split
        const waveH = viewMode === 'split' ? clipH * 0.48 : viewMode === 'waveform' ? clipH : 0;
        const spectroH = viewMode === 'split' ? clipH * 0.52 : viewMode === 'spectrogram' ? clipH : 0;
        const spectroY = viewMode === 'split' ? clipY + waveH : clipY;

        // 1. Spectrogram Area (if spectrogram or split)
        if (spectroH > 0) {
          const numCols = Math.min(Math.floor(clipWidth / 4), 80);
          const numRows = 16;
          const colW = clipWidth / numCols;
          const rowH = spectroH / numRows;

          for (let c = 0; c < numCols; c++) {
            const colX = clipStartX + c * colW;
            const progress = (c / numCols);
            for (let r = 0; r < numRows; r++) {
              // Higher rows are higher frequency
              const freqFrac = (numRows - 1 - r) / numRows;
              // Synthetic harmonic energy calculation
              const harmonic1 = Math.exp(-Math.pow((freqFrac - 0.2) / 0.1, 2));
              const harmonic2 = 0.7 * Math.exp(-Math.pow((freqFrac - 0.4) / 0.12, 2));
              const harmonic3 = 0.4 * Math.exp(-Math.pow((freqFrac - 0.7) / 0.15, 2));
              const modulation = 0.6 + 0.4 * Math.sin(progress * Math.PI * 6 + r * 0.5);
              const intensity = Math.min(1.0, (harmonic1 + harmonic2 + harmonic3) * modulation);

              // Magma color interpolation: dark purple -> magenta -> orange -> yellow
              let rCol = 0, gCol = 0, bCol = 4;
              if (intensity < 0.25) {
                const t = intensity / 0.25;
                rCol = Math.round(t * 81); gCol = Math.round(t * 18); bCol = Math.round(4 + t * 120);
              } else if (intensity < 0.6) {
                const t = (intensity - 0.25) / 0.35;
                rCol = Math.round(81 + t * 102); gCol = Math.round(18 + t * 37); bCol = Math.round(124 - t * 3);
              } else if (intensity < 0.85) {
                const t = (intensity - 0.6) / 0.25;
                rCol = Math.round(183 + t * 69); gCol = Math.round(55 + t * 82); bCol = Math.round(121 - t * 24);
              } else {
                const t = (intensity - 0.85) / 0.15;
                rCol = Math.round(252 + t * 3); gCol = Math.round(137 + t * 118); bCol = Math.round(97 + t * 158);
              }

              ctx.fillStyle = `rgb(${rCol}, ${gCol}, ${bCol})`;
              ctx.fillRect(colX, spectroY + r * rowH, colW + 0.5, rowH + 0.5);
            }
          }

          // In split mode, draw a subtle dividing rule between waveform & spectrogram
          if (viewMode === 'split') {
            ctx.strokeStyle = 'rgba(255, 255, 255, 0.2)';
            ctx.lineWidth = 1;
            ctx.beginPath();
            ctx.moveTo(clipStartX, spectroY);
            ctx.lineTo(clipStartX + clipWidth, spectroY);
            ctx.stroke();
          }
        }

        // 2. Waveform Area (if waveform or split)
        if (waveH > 0) {
          ctx.strokeStyle = isSelected ? '#38bdf8' : '#06b6d4';
          ctx.lineWidth = 1.5;
          ctx.beginPath();
          const midY = clipY + (waveH / 2);
          const waveSamples = Math.min(Math.floor(clipWidth / 3), 120);

          for (let s = 0; s < waveSamples; s++) {
            const sampleX = clipStartX + (s / waveSamples) * clipWidth;
            const angle = (s / waveSamples) * Math.PI * 8;
            const amp = Math.sin(angle) * 0.4 + Math.sin(angle * 2.3) * 0.2;
            const h = Math.abs(amp) * (waveH * 0.7);
            ctx.moveTo(sampleX, midY - h / 2);
            ctx.lineTo(sampleX, midY + h / 2);
          }
          ctx.stroke();
        }

        // Fade-In Curve & Shading
        if (clip.fade_in_secs > 0) {
          const fadeInW = clip.fade_in_secs * zoom;
          ctx.fillStyle = 'rgba(0, 0, 0, 0.35)';
          ctx.beginPath();
          ctx.moveTo(clipStartX, clipY);
          ctx.lineTo(clipStartX, clipY + clipH);
          ctx.lineTo(clipStartX + fadeInW, clipY);
          ctx.closePath();
          ctx.fill();

          ctx.strokeStyle = '#38bdf8';
          ctx.lineWidth = 1;
          ctx.beginPath();
          ctx.moveTo(clipStartX, clipY + clipH);
          ctx.quadraticCurveTo(clipStartX + fadeInW * 0.3, clipY + clipH * 0.3, clipStartX + fadeInW, clipY);
          ctx.stroke();
        }

        // Fade-Out Curve & Shading
        if (clip.fade_out_secs > 0) {
          const fadeOutW = clip.fade_out_secs * zoom;
          const fadeOutStart = clipStartX + clipWidth - fadeOutW;
          ctx.fillStyle = 'rgba(0, 0, 0, 0.35)';
          ctx.beginPath();
          ctx.moveTo(fadeOutStart, clipY);
          ctx.lineTo(clipStartX + clipWidth, clipY + clipH);
          ctx.lineTo(clipStartX + clipWidth, clipY);
          ctx.closePath();
          ctx.fill();

          ctx.strokeStyle = '#38bdf8';
          ctx.lineWidth = 1;
          ctx.beginPath();
          ctx.moveTo(fadeOutStart, clipY);
          ctx.quadraticCurveTo(fadeOutStart + fadeOutW * 0.7, clipY + clipH * 0.3, clipStartX + clipWidth, clipY + clipH);
          ctx.stroke();
        }

        // Volume Envelope line (Yellow/Amber)
        if (clip.envelope.length > 0) {
          ctx.strokeStyle = '#fbbf24';
          ctx.lineWidth = 1.2;
          ctx.beginPath();
          clip.envelope.forEach((pt, pIdx) => {
            const ptX = clipStartX + (pt.offset_secs * zoom);
            const ptY = clipY + clipH - (pt.gain * (clipH * 0.8));
            if (pIdx === 0) ctx.moveTo(ptX, ptY);
            else ctx.lineTo(ptX, ptY);
          });
          ctx.stroke();
        }

        // Pan Automation Envelope line (Cyan with Node markers)
        if (clip.pan_envelope && clip.pan_envelope.length > 0) {
          ctx.strokeStyle = '#06b6d4';
          ctx.lineWidth = 1.5;
          ctx.beginPath();
          clip.pan_envelope.forEach((pt, pIdx) => {
            const ptX = clipStartX + (pt.offset_secs * zoom);
            // Pan: -1 (L, top) to +1 (R, bottom)
            const ptY = clipY + (clipH / 2) + (pt.pan * (clipH * 0.35));
            if (pIdx === 0) ctx.moveTo(ptX, ptY);
            else ctx.lineTo(ptX, ptY);
          });
          ctx.stroke();

          // Draw Pan Nodes
          clip.pan_envelope.forEach((pt) => {
            const ptX = clipStartX + (pt.offset_secs * zoom);
            const ptY = clipY + (clipH / 2) + (pt.pan * (clipH * 0.35));
            ctx.fillStyle = '#ffffff';
            ctx.strokeStyle = '#06b6d4';
            ctx.lineWidth = 1.5;
            ctx.beginPath();
            ctx.arc(ptX, ptY, 3, 0, Math.PI * 2);
            ctx.fill();
            ctx.stroke();
          });
        }

        // Clip Title Bar & Slip badge
        ctx.fillStyle = isSelected ? '#38bdf8' : '#94a3b8';
        ctx.font = 'bold 10px monospace';
        ctx.fillText(clip.name, clipStartX + 8, clipY + 14);

        if (clip.slip_offset_secs && clip.slip_offset_secs > 0) {
          ctx.fillStyle = '#a855f7';
          ctx.font = '9px monospace';
          ctx.fillText(`slip: +${clip.slip_offset_secs.toFixed(2)}s`, clipStartX + 8, clipY + 26);
        }

        ctx.restore();

        // Clip Outline & Selection Ring
        ctx.strokeStyle = isSelected ? '#00f0ff' : 'rgba(255, 255, 255, 0.2)';
        ctx.lineWidth = isSelected ? 2 : 1;
        ctx.beginPath();
        ctx.roundRect(clipStartX, clipY, clipWidth, clipH, 6);
        ctx.stroke();

        // Slip drag ghost badge
        if (dragState && dragState.mode === 'slip_clip' && dragState.target.clip?.id === clip.id) {
          const deltaSecs = dragState.snappedTimeSecs - dragState.startTimeSecs;
          ctx.save();
          ctx.fillStyle = 'rgba(168, 85, 247, 0.9)';
          ctx.strokeStyle = '#d8b4fe';
          ctx.lineWidth = 1;
          const badgeX = clipStartX + clipWidth / 2 - 45;
          const badgeY = clipY + clipH / 2 - 10;
          ctx.beginPath();
          ctx.roundRect(badgeX, badgeY, 90, 20, 4);
          ctx.fill();
          ctx.stroke();

          ctx.fillStyle = '#ffffff';
          ctx.font = 'bold 10px monospace';
          ctx.textAlign = 'center';
          ctx.fillText(`⇄ ${deltaSecs >= 0 ? '+' : ''}${deltaSecs.toFixed(3)}s`, badgeX + 45, badgeY + 14);
          ctx.restore();
        }

        // Trim Handle indicators (left and right)
        ctx.fillStyle = isSelected ? '#00f0ff' : 'rgba(255, 255, 255, 0.4)';
        ctx.fillRect(clipStartX, clipY + clipH / 2 - 8, 3, 16);
        ctx.fillRect(clipStartX + clipWidth - 3, clipY + clipH / 2 - 8, 3, 16);
      });

      // 2D Spectral Selection Bounding Box on Track (if active)
      if (track.spectral_selection && (track.view_mode === 'spectrogram' || track.view_mode === 'split')) {
        const sel = track.spectral_selection;
        const selX1 = timeToPixels(sel.start_secs, zoom, scrollX, TRACK_HEADER_WIDTH);
        const selX2 = timeToPixels(sel.end_secs, zoom, scrollX, TRACK_HEADER_WIDTH);
        const selW = Math.max(4, selX2 - selX1);

        const spectroY = track.view_mode === 'split' ? trackTop + (TRACK_HEIGHT * 0.48) : trackTop + 6;
        const spectroH = track.view_mode === 'split' ? TRACK_HEIGHT * 0.52 - 12 : TRACK_HEIGHT - 12;

        // Logarithmic frequency to Y within track spectrogram bounds
        const minF = 20.0;
        const maxF = 20000.0;
        const f1Y = spectroY + (1.0 - Math.log(sel.f_max_hz / minF) / Math.log(maxF / minF)) * spectroH;
        const f2Y = spectroY + (1.0 - Math.log(sel.f_min_hz / minF) / Math.log(maxF / minF)) * spectroH;
        const selH = Math.max(4, f2Y - f1Y);

        ctx.save();
        ctx.fillStyle = 'rgba(6, 182, 212, 0.22)';
        ctx.fillRect(selX1, f1Y, selW, selH);

        ctx.strokeStyle = '#22d3ee';
        ctx.lineWidth = 1.5;
        ctx.setLineDash([3, 2]);
        ctx.strokeRect(selX1, f1Y, selW, selH);

        // HUD Readout badge
        ctx.setLineDash([]);
        ctx.fillStyle = 'rgba(15, 23, 42, 0.9)';
        ctx.strokeStyle = 'rgba(6, 182, 212, 0.6)';
        ctx.lineWidth = 1;
        const hudW = 120;
        const hudH = 16;
        const hudX = Math.max(TRACK_HEADER_WIDTH + 4, selX1);
        const hudY = Math.max(trackTop + 2, f1Y - 18);
        ctx.beginPath();
        ctx.roundRect(hudX, hudY, hudW, hudH, 3);
        ctx.fill();
        ctx.stroke();

        ctx.fillStyle = '#38bdf8';
        ctx.font = 'bold 9px monospace';
        ctx.textAlign = 'center';
        ctx.fillText(`fc:${Math.round(sel.center_freq_hz)}Hz Q:${sel.q_factor.toFixed(2)}`, hudX + hudW / 2, hudY + 11);
        ctx.textAlign = 'left';
        ctx.restore();
      }
    });

    // 5. In-Out Selection Shading
    if (timeline.selection) {
      const selX = timeToPixels(timeline.selection.start_secs, zoom, scrollX, TRACK_HEADER_WIDTH);
      const selW = Math.max(2, (timeline.selection.end_secs - timeline.selection.start_secs) * zoom);
      ctx.fillStyle = 'rgba(6, 182, 212, 0.12)';
      ctx.fillRect(selX, RULER_HEIGHT, selW, height - RULER_HEIGHT);
      ctx.strokeStyle = 'rgba(6, 182, 212, 0.4)';
      ctx.strokeRect(selX, RULER_HEIGHT, selW, height - RULER_HEIGHT);
    }

    // 6. Timecode Ruler & Markers
    ctx.fillStyle = '#0f172a';
    ctx.fillRect(0, 0, width, RULER_HEIGHT);
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.12)';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, RULER_HEIGHT);
    ctx.lineTo(width, RULER_HEIGHT);
    ctx.stroke();

    // Ruler ticks & timecode labels
    for (let t = firstGridSec; t <= endTimeSecs; t += gridStepSecs) {
      const x = timeToPixels(t, zoom, scrollX, TRACK_HEADER_WIDTH);
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.3)';
      ctx.beginPath();
      ctx.moveTo(x, RULER_HEIGHT - 10);
      ctx.lineTo(x, RULER_HEIGHT);
      ctx.stroke();

      ctx.fillStyle = '#94a3b8';
      ctx.font = '10px monospace';
      ctx.fillText(formatTimecode(t), x + 4, RULER_HEIGHT - 12);
    }

    // Loop Region on Ruler
    if (timeline.loop_range) {
      const loopX = timeToPixels(timeline.loop_range.start_secs, zoom, scrollX, TRACK_HEADER_WIDTH);
      const loopW = (timeline.loop_range.end_secs - timeline.loop_range.start_secs) * zoom;
      ctx.fillStyle = 'rgba(245, 158, 11, 0.25)';
      ctx.fillRect(loopX, 0, loopW, RULER_HEIGHT);
      ctx.strokeStyle = '#f59e0b';
      ctx.lineWidth = 1.5;
      ctx.strokeRect(loopX, 0, loopW, RULER_HEIGHT);
    }

    // Markers on Ruler
    timeline.markers.forEach((m) => {
      const mX = timeToPixels(m.time_secs, zoom, scrollX, TRACK_HEADER_WIDTH);
      ctx.fillStyle = '#a855f7';
      ctx.beginPath();
      ctx.moveTo(mX, 0);
      ctx.lineTo(mX + 6, 6);
      ctx.lineTo(mX + 6, RULER_HEIGHT - 6);
      ctx.lineTo(mX, RULER_HEIGHT);
      ctx.lineTo(mX - 6, RULER_HEIGHT - 6);
      ctx.lineTo(mX - 6, 6);
      ctx.closePath();
      ctx.fill();

      ctx.fillStyle = '#ffffff';
      ctx.font = 'bold 9px system-ui';
      ctx.fillText(m.label, mX + 8, RULER_HEIGHT - 12);
    });

    // 7. Playhead (Bright Cyan Line & Top Triangle Handle)
    const playheadX = timeToPixels(currentTimeSecs, zoom, scrollX, TRACK_HEADER_WIDTH);
    if (playheadX >= TRACK_HEADER_WIDTH && playheadX <= width) {
      // Triangle Flag
      ctx.fillStyle = isPlaying ? '#22c55e' : '#00f0ff';
      ctx.beginPath();
      ctx.moveTo(playheadX - 6, 0);
      ctx.lineTo(playheadX + 6, 0);
      ctx.lineTo(playheadX, 10);
      ctx.closePath();
      ctx.fill();

      // Playhead vertical beam
      ctx.strokeStyle = isPlaying ? '#22c55e' : '#00f0ff';
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.moveTo(playheadX, 0);
      ctx.lineTo(playheadX, height);
      ctx.stroke();
    }

    // 8. Snap Line indicator if dragging and snapped
    if (dragState && dragState.snappedTimeSecs !== dragState.currentTimeSecs) {
      const snapLineX = timeToPixels(dragState.snappedTimeSecs, zoom, scrollX, TRACK_HEADER_WIDTH);
      ctx.strokeStyle = '#eab308';
      ctx.setLineDash([4, 4]);
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(snapLineX, 0);
      ctx.lineTo(snapLineX, height);
      ctx.stroke();
      ctx.setLineDash([]);
    }
  }, [timeline, currentTimeSecs, isPlaying, zoom, scrollX, scrollY, dragState, canvasSize]);

  // Pointer Interaction Handlers
  const handlePointerDown = (e: React.PointerEvent<HTMLCanvasElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    // 1. Check Track Header View Mode buttons click
    if (x < TRACK_HEADER_WIDTH && y > RULER_HEIGHT) {
      const trackIndex = Math.floor((y - RULER_HEIGHT + scrollY) / TRACK_HEIGHT);
      if (trackIndex >= 0 && trackIndex < timeline.tracks.length) {
        const track = timeline.tracks[trackIndex];
        const trackTop = RULER_HEIGHT + (trackIndex * TRACK_HEIGHT) - scrollY;
        const relativeY = y - trackTop;
        if (relativeY >= 32 && relativeY <= 48) {
          if (x >= 86 && x <= 104) {
            onDispatchCommand({ type: 'set_track_view_mode', trackId: track.id, viewMode: 'waveform' });
            return;
          } else if (x >= 108 && x <= 126) {
            onDispatchCommand({ type: 'set_track_view_mode', trackId: track.id, viewMode: 'spectrogram' });
            return;
          } else if (x >= 130 && x <= 168) {
            onDispatchCommand({ type: 'set_track_view_mode', trackId: track.id, viewMode: 'split' });
            return;
          }
        }
      }
    }

    const target = hitTest(x, y, timeline, viewport);

    if (target.clip) {
      onSelectClip(target.clip.id);
    } else if (target.kind === 'track_empty') {
      onSelectClip(null);
    }

    if (target.kind === 'ruler') {
      onSeek(target.timeSecs);
    }

    const session = createDragSession(
      x,
      y,
      target,
      zoom,
      scrollX,
      TRACK_HEADER_WIDTH,
      e.altKey
    );
    setDragState(session);
    e.currentTarget.setPointerCapture(e.pointerId);
  };

  const handlePointerMove = (e: React.PointerEvent<HTMLCanvasElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    if (dragState) {
      const snapCandidates = getSnapCandidates();
      const updated = updateDragSession(
        dragState,
        x,
        y,
        snapCandidates,
        zoom,
        scrollX,
        TRACK_HEADER_WIDTH
      );
      setDragState(updated);

      if (dragState.mode === 'scrub_playhead') {
        onSeek(updated.snappedTimeSecs);
      }
    } else {
      const target = hitTest(x, y, timeline, viewport);
      setHoverTarget(target);
    }
  };

  const handlePointerUp = (e: React.PointerEvent<HTMLCanvasElement>) => {
    if (dragState) {
      const command = resolveDragCommand(dragState);
      if (command) {
        onDispatchCommand(command);
      }
      setDragState(null);
    }
    try {
      e.currentTarget.releasePointerCapture(e.pointerId);
    } catch {
      // Ignored
    }
  };

  // Wheel zoom and pan
  const handleWheel = (e: React.WheelEvent<HTMLCanvasElement>) => {
    if (e.ctrlKey) {
      // Horizontal zoom at mouse position
      e.preventDefault();
      const zoomFactor = e.deltaY < 0 ? 1.15 : 0.85;
      const newZoom = Math.max(10, Math.min(1000, zoom * zoomFactor));
      if (onZoomChange) onZoomChange(newZoom);
    } else if (e.shiftKey) {
      // Horizontal pan
      e.preventDefault();
      setScrollX((prev) => Math.max(0, prev + e.deltaY));
    } else {
      // Vertical pan
      setScrollY((prev) => Math.max(0, prev + e.deltaY));
    }
  };

  // Cursor style derived from hover
  const getCursorStyle = (): string => {
    if (dragState) {
      if (dragState.mode === 'move_clip') return 'grabbing';
      if (dragState.mode.startsWith('trim_')) return 'col-resize';
      return 'crosshair';
    }
    if (!hoverTarget) return 'default';
    switch (hoverTarget.kind) {
      case 'clip_trim_left':
      case 'clip_trim_right':
      case 'loop_start':
      case 'loop_end':
        return 'col-resize';
      case 'clip_fade_in':
      case 'clip_fade_out':
        return 'nwse-resize';
      case 'clip_body':
        return 'grab';
      case 'ruler':
        return 'pointer';
      default:
        return 'default';
    }
  };

  return (
    <div
      ref={containerRef}
      className="w-full h-full relative overflow-hidden select-none bg-[#090d16]"
      style={{ cursor: getCursorStyle() }}
    >
      <canvas
        ref={canvasRef}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onWheel={handleWheel}
        className="w-full h-full block touch-none"
      />
    </div>
  );
}
