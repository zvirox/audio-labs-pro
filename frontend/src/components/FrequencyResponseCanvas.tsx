/**
 * Native Canvas 2D magnitude/phase plot for the active biquad chain.
 * Uses the shared Linear UI tokens; no chart library or decorative effects.
 */

import { useEffect, useRef } from 'react';
import type { BiquadResponseDto } from '../ipc/types';
import { readLinearCanvasTokens, useLinearCanvasTheme } from '../utils/linearCanvasTheme';

export interface FrequencyResponseCanvasProps {
  response: BiquadResponseDto | null;
  height?: number;
}

const DB_MIN = -48;
const DB_MAX = 12;

export function FrequencyResponseCanvas({
  response,
  height = 180,
}: FrequencyResponseCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const themeTick = useLinearCanvasTheme();

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const draw = () => {
      const parent = canvas.parentElement;
      const width = Math.max(240, parent?.clientWidth ?? 640);
      const dpr = window.devicePixelRatio || 1;
      canvas.width = Math.floor(width * dpr);
      canvas.height = Math.floor(height * dpr);
      canvas.style.width = `${width}px`;
      canvas.style.height = `${height}px`;
      const ctx = canvas.getContext('2d');
      if (!ctx) return;
      ctx.scale(dpr, dpr);
      const tokens = readLinearCanvasTokens();
      const minHz = 20;
      const maxHz = 20_000;
      const pad = { left: 38, right: 12, top: 12, bottom: 24 };
      const plotW = width - pad.left - pad.right;
      const plotH = height - pad.top - pad.bottom;
      const xFor = (hz: number) =>
        pad.left + (Math.log10(Math.max(minHz, hz) / minHz) / Math.log10(maxHz / minHz)) * plotW;
      const yDb = (db: number) =>
        pad.top + ((DB_MAX - Math.max(DB_MIN, Math.min(DB_MAX, db))) / (DB_MAX - DB_MIN)) * plotH;
      const yPhase = (degrees: number) =>
        pad.top + ((180 - Math.max(-180, Math.min(180, degrees))) / 360) * plotH;

      ctx.clearRect(0, 0, width, height);
      ctx.lineWidth = 1;
      ctx.strokeStyle = tokens.border;
      ctx.fillStyle = tokens.textMuted;
      ctx.font = '10px Inter, sans-serif';

      for (const db of [0, -12, -24, -36, -48]) {
        const y = yDb(db);
        ctx.beginPath();
        ctx.moveTo(pad.left, y);
        ctx.lineTo(width - pad.right, y);
        ctx.stroke();
        ctx.fillText(`${db} dB`, 2, y + 3);
      }
      for (const hz of [100, 1_000, 10_000]) {
        const x = xFor(hz);
        ctx.beginPath();
        ctx.moveTo(x, pad.top);
        ctx.lineTo(x, height - pad.bottom);
        ctx.stroke();
        ctx.fillText(hz >= 1_000 ? `${hz / 1_000}k` : `${hz}`, x - 8, height - 8);
      }

      if (!response || response.frequencies_hz.length === 0) {
        ctx.fillStyle = tokens.textSecondary;
        ctx.fillText('No response data', pad.left + 8, pad.top + 20);
        return;
      }

      ctx.lineWidth = 2;
      ctx.strokeStyle = tokens.primary;
      ctx.beginPath();
      response.frequencies_hz.forEach((hz, index) => {
        const x = xFor(hz);
        const y = yDb(response.magnitude_db[index] ?? 0);
        if (index === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      });
      ctx.stroke();

      ctx.lineWidth = 1;
      ctx.strokeStyle = tokens.sky;
      ctx.beginPath();
      response.frequencies_hz.forEach((hz, index) => {
        const x = xFor(hz);
        const y = yPhase(response.phase_deg[index] ?? 0);
        if (index === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      });
      ctx.stroke();

      ctx.fillStyle = tokens.textMuted;
      ctx.fillText('magnitude', width - 118, pad.top + 10);
      ctx.fillText('phase', width - 54, pad.top + 10);
    };

    draw();
    const observer = new ResizeObserver(draw);
    if (canvas.parentElement) observer.observe(canvas.parentElement);
    return () => observer.disconnect();
  }, [response, height, themeTick]);

  return <canvas ref={canvasRef} aria-label="EQ magnitude and phase response" role="img" />;
}
