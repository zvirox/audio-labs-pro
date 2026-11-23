/**
 * Audio Labs Pro - Timeline & Waveform Panel
 *
 * Studio multitrack timeline editor slot with peak pyramid waveform visualization,
 * zoom controls, selection range editing, and 1-hour benchmark testing mode.
 */

import { useState, useEffect } from 'react';
import { WaveformCanvas } from '../waveform/WaveformCanvas';
import { Button } from '../kit/Button';
import { Toggle } from '../kit/Toggle';
import { getWaveformPeaks, getBenchmark1HourPeaks } from '../ipc/client';
import type { WaveformPeaksDto } from '../ipc/types';
import { Activity, Play, Pause, Repeat } from 'lucide-react';

export interface TimelineWaveformPanelProps {
  currentTime: number;
  duration: number;
  isPlaying: boolean;
  loopEnabled: boolean;
  loopStart: number;
  loopEnd: number;
  onSeek: (timeSecs: number) => void;
  onTogglePlay: () => void;
  onToggleLoop: () => void;
}

export function TimelineWaveformPanel({
  currentTime,
  duration,
  isPlaying,
  loopEnabled,
  loopStart,
  loopEnd,
  onSeek,
  onTogglePlay,
  onToggleLoop,
}: TimelineWaveformPanelProps) {
  const [peaks, setPeaks] = useState<WaveformPeaksDto | null>(null);
  const [is1HourBenchmark, setIs1HourBenchmark] = useState<boolean>(false);
  const [autoscroll, setAutoscroll] = useState<boolean>(true);
  const [selectionRange, setSelectionRange] = useState<{ start: number; end: number }>({
    start: 2.0,
    end: 6.0,
  });

  // Load peaks based on mode (session vs 1-hour benchmark)
  useEffect(() => {
    let mounted = true;

    if (is1HourBenchmark) {
      getBenchmark1HourPeaks().then((data) => {
        if (mounted) setPeaks(data);
      });
    } else {
      getWaveformPeaks().then((data) => {
        if (mounted) setPeaks(data);
      });
    }

    return () => {
      mounted = false;
    };
  }, [is1HourBenchmark]);

  const activeDuration = is1HourBenchmark ? 3600.0 : duration;

  const handleSelectRange = (start: number, end: number) => {
    setSelectionRange({ start, end });
  };

  const formatTime = (secs: number): string => {
    const mins = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    const ms = Math.floor((secs % 1) * 100);
    return `${mins.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}.${ms.toString().padStart(2, '0')}`;
  };

  return (
    <div className="flex flex-col h-full w-full gap-2 text-xs">
      {/* Top Toolbar */}
      <div className="flex items-center justify-between px-2 py-1.5 bg-[var(--color-surface-2)] rounded border border-[var(--color-border)]">
        <div className="flex items-center gap-2">
          <Button
            variant="ghost"
            size="sm"
            onClick={onTogglePlay}
            aria-label={isPlaying ? 'Pause' : 'Play'}
          >
            {isPlaying ? <Pause className="w-4 h-4" /> : <Play className="w-4 h-4" />}
          </Button>

          <Button
            variant={loopEnabled ? 'primary' : 'ghost'}
            size="sm"
            onClick={onToggleLoop}
            aria-label="Toggle loop playback"
          >
            <Repeat className="w-4 h-4" />
          </Button>

          <div className="h-4 w-px bg-[var(--color-surface-2)] mx-1" />

          <span className="font-mono text-[var(--color-accent)] font-medium px-2 py-0.5 rounded bg-black/40 border border-[var(--color-border-strong)]">
            {formatTime(currentTime)}
          </span>
          <span className="text-[var(--color-fg-muted)] font-mono">/ {formatTime(activeDuration)}</span>
        </div>

        {/* Viewport & Benchmark Controls */}
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-1.5">
            <Toggle
              checked={autoscroll}
              onChange={setAutoscroll}
              label="Autoscroll"
            />
          </div>

          <div className="h-4 w-px bg-[var(--color-surface-2)]" />

          {/* 1-Hour Benchmark Gate Switch */}
          <button
            type="button"
            onClick={() => setIs1HourBenchmark(!is1HourBenchmark)}
            className={`flex items-center gap-1.5 px-2 py-1 rounded transition-colors border ${
 is1HourBenchmark
 ? 'bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] border-[var(--color-border-strong)] shadow-sm'
 : 'bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] hover:text-[var(--color-fg)] border-[var(--color-border)]'
 }`}
          >
            <Activity className="w-4 h-4 text-[var(--color-fg-muted)]" />
            <span className="font-medium">
              {is1HourBenchmark ? '1-Hour Benchmark (Active)' : 'Test 1-Hour Gate'}
            </span>
          </button>
        </div>
      </div>

      {/* Main Waveform Display */}
      <div className="flex-1 min-h-[140px] flex flex-col justify-center">
        {peaks ? (
          <WaveformCanvas
            peaks={peaks}
            currentTime={currentTime}
            duration={activeDuration}
            isPlaying={isPlaying}
            loopEnabled={loopEnabled}
            loopStart={loopStart}
            loopEnd={loopEnd}
            selectionStart={selectionRange.start}
            selectionEnd={selectionRange.end}
            onSeek={onSeek}
            onSelectRange={handleSelectRange}
            autoscroll={autoscroll}
            height={160}
          />
        ) : (
          <div className="flex flex-col items-center justify-center h-40 bg-[var(--color-surface-2)] rounded border border-[var(--color-border)] text-[var(--color-fg-muted)]">
            <div className="w-4 h-4 border-2 border-[var(--color-border-strong)] border-t-cyan-400 rounded-full animate-spin mb-2" />
            <span>Loading peak pyramid...</span>
          </div>
        )}
      </div>

      {/* Bottom Status / Selection Readout */}
      <div className="flex items-center justify-between px-2 text-[11px] text-[var(--color-fg-muted)] font-mono">
        <div>
          Selection: {formatTime(selectionRange.start)} – {formatTime(selectionRange.end)} (
          {(selectionRange.end - selectionRange.start).toFixed(2)}s)
        </div>
        <div>
          Pyramid: 3 levels (256, 1024, 4096 spl) · {is1HourBenchmark ? '675,000 buckets' : '3,750 buckets'}
        </div>
      </div>
    </div>
  );
}
