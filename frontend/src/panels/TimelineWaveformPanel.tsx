import { useMemo, useState } from 'react';
import { Repeat, ZoomIn, ZoomOut } from 'lucide-react';
import { WaveformCanvas } from '../waveform/WaveformCanvas';
import { formatTimecode } from '../timeline/timelineMath';
import { Button } from '../kit/Button';
import type { WaveformPeaksDto } from '../ipc/types';

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
  onToggleLoop,
}: TimelineWaveformPanelProps) {
  const [zoom, setZoom] = useState(1);

  const peaks: WaveformPeaksDto = useMemo(() => {
    const sampleRate = 48000;
    const totalFrames = Math.max(sampleRate, Math.round(duration * sampleRate));
    const bucketSamples = Math.max(256, Math.round(2048 / zoom));
    const count = Math.max(120, Math.floor(totalFrames / bucketSamples));
    const pairs: [number, number][] = Array.from({ length: count }, (_, idx) => {
      const t = idx / count;
      const amp = Math.min(0.95, 0.22 + 0.58 * Math.abs(Math.sin(t * 28) * Math.cos(t * 9)));
      const q = Math.round(amp * 32767);
      return [-q, q];
    });
    return {
      sample_rate: sampleRate,
      frames: totalFrames,
      duration_secs: Math.max(1, duration),
      levels: [{ bucket_samples: bucketSamples, pairs }],
    };
  }, [duration, zoom]);

  return (
    <div className="flex flex-col gap-3 text-[var(--text)]">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <span className="rounded border border-[var(--border)] bg-[var(--bg-surface-2)] px-2 py-0.5 font-mono text-[11px] font-medium tabular-nums">
            {formatTimecode(currentTime)} / {formatTimecode(duration)}
          </span>
          <Button
            size="xs"
            variant={loopEnabled ? 'primary' : 'secondary'}
            leftIcon={<Repeat className="h-3 w-3" />}
            onClick={onToggleLoop}
          >
            Loop {formatTimecode(loopStart)}–{formatTimecode(loopEnd)}
          </Button>
        </div>

        <div className="flex items-center gap-1">
          <Button
            size="xs"
            variant="secondary"
            leftIcon={<ZoomOut className="h-3 w-3" />}
            onClick={() => setZoom((z) => Math.max(1, z - 0.5))}
          >
            Zoom Out
          </Button>
          <Button
            size="xs"
            variant="secondary"
            leftIcon={<ZoomIn className="h-3 w-3" />}
            onClick={() => setZoom((z) => Math.min(4, z + 0.5))}
          >
            Zoom In
          </Button>
        </div>
      </div>

      <WaveformCanvas
        peaks={peaks}
        duration={Math.max(1, duration)}
        currentTime={currentTime}
        isPlaying={isPlaying}
        loopEnabled={loopEnabled}
        loopStart={loopStart}
        loopEnd={loopEnd}
        onSeek={onSeek}
        height={120}
      />
    </div>
  );
}
