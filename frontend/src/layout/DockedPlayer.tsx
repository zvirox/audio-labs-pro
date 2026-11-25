import { useState } from 'react';
import {
  Circle,
  Pause,
  Play,
  Repeat,
  Square,
  Volume2,
  VolumeX,
} from 'lucide-react';
import { Meter } from '../kit/Meter';
import { Slider } from '../kit/Slider';
import { formatTimecode } from '../timeline/timelineMath';

export interface DockedPlayerProps {
  durationSecs?: number;
  sampleRate?: number;
  playing?: boolean;
  currentTime?: number;
  volume?: number;
  loopEnabled?: boolean;
  peakL?: number;
  peakR?: number;
  isClipping?: boolean;
  onTogglePlay?: () => void;
  onStop?: () => void;
  onSeek?: (secs: number) => void;
  onSetVolume?: (vol: number) => void;
  onToggleLoop?: () => void;
}

export function DockedPlayer({
  durationSecs = 20.0,
  sampleRate = 48000,
  playing,
  currentTime,
  volume = 1.0,
  loopEnabled,
  peakL = 0.0,
  peakR = 0.0,
  isClipping = false,
  onTogglePlay,
  onStop,
  onSeek,
  onSetVolume,
  onToggleLoop,
}: DockedPlayerProps) {
  const [localPlaying, setLocalPlaying] = useState(false);
  const [isRecording, setIsRecording] = useState(false);
  const [localLooping, setLocalLooping] = useState(false);
  const [localTime, setLocalTime] = useState(0.0);
  const [localVolume, setLocalVolume] = useState(volume);
  const [isMuted, setIsMuted] = useState(false);

  const activePlaying = playing !== undefined ? playing : localPlaying;
  const activeTime = currentTime !== undefined ? currentTime : localTime;
  const activeLooping = loopEnabled !== undefined ? loopEnabled : localLooping;
  const activeVolume = volume !== undefined ? volume : localVolume;

  const handlePlayToggle = () => {
    if (onTogglePlay) {
      onTogglePlay();
    } else {
      setLocalPlaying(!localPlaying);
    }
  };

  const handleStop = () => {
    if (onStop) {
      onStop();
    } else {
      setLocalPlaying(false);
      setLocalTime(0.0);
    }
  };

  const handleSeek = (newTime: number) => {
    if (onSeek) {
      onSeek(newTime);
    } else {
      setLocalTime(newTime);
    }
  };

  const handleVolumeChange = (newVol: number) => {
    if (onSetVolume) {
      onSetVolume(newVol);
    } else {
      setLocalVolume(newVol);
    }
    if (isMuted) setIsMuted(false);
  };

  const handleLoopToggle = () => {
    if (onToggleLoop) {
      onToggleLoop();
    } else {
      setLocalLooping(!localLooping);
    }
  };

  return (
    <footer
      aria-label="Global Transport Bar"
      className="flex h-14 shrink-0 items-center justify-between gap-4 border-t border-[var(--border)] bg-[var(--bg-surface)] px-4 text-[var(--text)] select-none"
    >
      {/* Left: Transport Controls */}
      <div className="flex items-center gap-1.5">
        <button
          type="button"
          onClick={handleStop}
          aria-label="Stop"
          title="Stop (Shift+Space)"
          className="inline-flex h-8 w-8 items-center justify-center rounded-md border border-[var(--border)] bg-[var(--bg-surface-2)] text-[var(--text-muted)] hover:border-[var(--border-strong)] hover:text-[var(--text)] transition-colors duration-[var(--duration-quick)] cursor-pointer"
        >
          <Square className="h-3.5 w-3.5" />
        </button>

        <button
          type="button"
          onClick={handlePlayToggle}
          aria-label={activePlaying ? 'Pause' : 'Play'}
          title="Play / Pause (Space)"
          className="inline-flex h-8 w-9 items-center justify-center rounded-md border border-[var(--accent-blue)] bg-[var(--accent-blue)] text-[var(--text-on-accent)] hover:bg-[var(--accent-blue-hover)] transition-colors duration-[var(--duration-quick)] cursor-pointer"
        >
          {activePlaying ? (
            <Pause className="h-4 w-4 fill-current" />
          ) : (
            <Play className="ml-0.5 h-4 w-4 fill-current" />
          )}
        </button>

        <button
          type="button"
          onClick={() => setIsRecording((prev) => !prev)}
          aria-label="Arm recording"
          title="Record Arm"
          className={`inline-flex h-8 w-8 items-center justify-center rounded-md border transition-colors duration-[var(--duration-quick)] cursor-pointer ${
            isRecording
              ? 'border-[var(--accent-red)] bg-[var(--tint-red)] text-[var(--accent-red)]'
              : 'border-[var(--border)] bg-[var(--bg-surface-2)] text-[var(--text-muted)] hover:border-[var(--border-strong)] hover:text-[var(--accent-red)]'
          }`}
        >
          <Circle className="h-3.5 w-3.5 fill-current" />
        </button>

        <button
          type="button"
          onClick={handleLoopToggle}
          aria-label="Toggle Loop"
          title="Loop (L)"
          className={`inline-flex h-8 w-8 items-center justify-center rounded-md border transition-colors duration-[var(--duration-quick)] cursor-pointer ${
            activeLooping
              ? 'border-[var(--accent-blue)] bg-[var(--tint-blue)] text-[var(--accent-blue)]'
              : 'border-[var(--border)] bg-[var(--bg-surface-2)] text-[var(--text-muted)] hover:border-[var(--border-strong)] hover:text-[var(--text)]'
          }`}
        >
          <Repeat className="h-3.5 w-3.5" />
        </button>
      </div>

      {/* Center: Timecode & Scrub Bar */}
      <div className="flex min-w-0 flex-1 items-center gap-3">
        <span className="w-20 shrink-0 text-right font-mono text-[12px] font-medium text-[var(--text)] tabular-nums">
          {formatTimecode(activeTime)}
        </span>
        <Slider
          value={activeTime}
          min={0}
          max={Math.max(1, durationSecs)}
          step={0.05}
          ariaLabel="Playback timeline scrubber"
          onChange={handleSeek}
          className="flex-1"
        />
        <span className="w-20 shrink-0 font-mono text-[12px] text-[var(--text-muted)] tabular-nums">
          {formatTimecode(durationSecs)}
        </span>
      </div>

      {/* Right: Meter, Sample Rate & Master Volume */}
      <div className="hidden items-center gap-4 md:flex">
        <div className="flex items-center gap-2">
          <Meter
            leftLevel={isMuted ? 0 : peakL}
            rightLevel={isMuted ? 0 : peakR}
            className="w-28"
          />
          {isClipping && (
            <span className="rounded border border-[var(--accent-red)] bg-[var(--tint-red)] px-1.5 py-0.5 text-[10px] font-semibold text-[var(--accent-red)]">
              CLIP
            </span>
          )}
        </div>

        <span className="rounded border border-[var(--border)] bg-[var(--bg-surface-2)] px-2 py-1 text-[11px] font-medium text-[var(--text-muted)] tabular-nums">
          {(sampleRate / 1000).toFixed(1)} kHz
        </span>

        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => setIsMuted((prev) => !prev)}
            aria-label={isMuted ? 'Unmute master output' : 'Mute master output'}
            className="inline-flex h-7 w-7 items-center justify-center rounded-md text-[var(--text-muted)] hover:bg-[var(--bg-surface-2)] hover:text-[var(--text)] cursor-pointer"
          >
            {isMuted || activeVolume === 0 ? (
              <VolumeX className="h-3.5 w-3.5 text-[var(--accent-red)]" />
            ) : (
              <Volume2 className="h-3.5 w-3.5" />
            )}
          </button>
          <Slider
            value={isMuted ? 0 : activeVolume}
            min={0}
            max={1}
            step={0.01}
            ariaLabel="Master output volume"
            onChange={handleVolumeChange}
            className="w-24"
          />
        </div>
      </div>
    </footer>
  );
}
