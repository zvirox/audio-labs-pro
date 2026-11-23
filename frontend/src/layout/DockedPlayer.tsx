import { useState, useEffect } from 'react';
import {
  Play,
  Pause,
  Square,
  Circle,
  Repeat,
  Volume2,
  VolumeX,
} from 'lucide-react';
import { Button } from '../kit/Button';
import { Slider } from '../kit/Slider';
import { Meter } from '../kit/Meter';

interface DockedPlayerProps {
  durationSecs?: number;
  sampleRate?: number;
}

export function DockedPlayer({
  durationSecs = 20.0,
  sampleRate = 48000,
}: DockedPlayerProps) {
  const [isPlaying, setIsPlaying] = useState(false);
  const [isRecording, setIsRecording] = useState(false);
  const [isLooping, setIsLooping] = useState(false);
  const [currentTime, setCurrentTime] = useState(4.25);
  const [volume, setVolume] = useState(0.85);
  const [isMuted, setIsMuted] = useState(false);

  // Meter simulation during playback
  const [leftMeter, setLeftMeter] = useState(0.0);
  const [rightMeter, setRightMeter] = useState(0.0);

  useEffect(() => {
    let timer: ReturnType<typeof setInterval>;
    if (isPlaying) {
      timer = setInterval(() => {
        setCurrentTime((prev) => {
          const next = prev + 0.1;
          if (next >= durationSecs) {
            return isLooping ? 0 : durationSecs;
          }
          return next;
        });

        // Simulate reactive audio energy
        const energyL = Math.min(0.95, Math.random() * 0.4 + (isMuted ? 0 : volume * 0.5));
        const energyR = Math.min(0.95, Math.random() * 0.4 + (isMuted ? 0 : volume * 0.48));
        setLeftMeter(energyL);
        setRightMeter(energyR);
      }, 100);
    } else {
      setLeftMeter(0.0);
      setRightMeter(0.0);
    }

    return () => clearInterval(timer);
  }, [isPlaying, isLooping, durationSecs, volume, isMuted]);

  // Space key play/pause
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (
        (e.target as HTMLElement).tagName === 'INPUT' ||
        (e.target as HTMLElement).tagName === 'TEXTAREA'
      ) {
        return;
      }
      if (e.code === 'Space') {
        e.preventDefault();
        setIsPlaying((prev) => !prev);
      }
    }
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  const formatTimecode = (seconds: number) => {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    const millis = Math.floor((seconds % 1) * 100);
    return `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}.${millis.toString().padStart(2, '0')}`;
  };

  return (
    <footer
      className="h-16 border-t border-[var(--color-border)] bg-[var(--color-surface)] px-4 flex items-center justify-between gap-4 z-30 select-none flex-shrink-0"
      aria-label="Transport and Playback Bar"
    >
      {/* Left: Transport Buttons */}
      <div className="flex items-center gap-2 flex-shrink-0">
        {/* Play/Pause */}
        <Button
          variant="primary"
          size="sm"
          onClick={() => setIsPlaying(!isPlaying)}
          leftIcon={
            isPlaying ? (
              <Pause className="w-4 h-4 fill-current" />
            ) : (
              <Play className="w-4 h-4 fill-current ml-0.5" />
            )
          }
          title="Play / Pause transport (Space)"
          aria-label={isPlaying ? 'Pause' : 'Play'}
        >
          {isPlaying ? 'Pause' : 'Play'}
        </Button>

        {/* Stop */}
        <Button
          variant="secondary"
          size="sm"
          onClick={() => {
            setIsPlaying(false);
            setCurrentTime(0);
          }}
          leftIcon={<Square className="w-3.5 h-3.5" />}
          title="Stop and return to 0:00"
          aria-label="Stop"
        />

        {/* Record */}
        <Button
          variant={isRecording ? 'danger' : 'secondary'}
          size="sm"
          onClick={() => setIsRecording(!isRecording)}
          leftIcon={<Circle className={`w-3.5 h-3.5 ${isRecording ? 'fill-current' : 'text-red-400'}`} />}
          title="Arm recording (R)"
          aria-label="Record"
        />

        {/* Loop */}
        <Button
          variant={isLooping ? 'primary' : 'ghost'}
          size="sm"
          onClick={() => setIsLooping(!isLooping)}
          leftIcon={<Repeat className="w-3.5 h-3.5" />}
          title="Toggle loop playback (L)"
          aria-label="Loop"
        />
      </div>

      {/* Center: Timecode & Timeline Scrubber */}
      <div className="flex-1 max-w-2xl flex items-center gap-3">
        {/* Digital Clock readout */}
        <div className="px-2.5 py-1 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border)] font-mono text-xs font-bold text-[var(--color-accent)] tracking-wider">
          {formatTimecode(currentTime)}
        </div>

        {/* Interactive scrubber */}
        <div className="flex-1">
          <Slider
            min={0}
            max={durationSecs}
            step={0.01}
            value={currentTime}
            onChange={setCurrentTime}
            formatValue={(v) => `${formatTimecode(v)} / ${formatTimecode(durationSecs)}`}
          />
        </div>

        <div className="text-[11px] font-mono text-[var(--color-fg-subtle)] whitespace-nowrap hidden sm:block">
          {formatTimecode(durationSecs)} &bull; {sampleRate / 1000}kHz
        </div>
      </div>

      {/* Right: Master Output Volume & Stereo Meters */}
      <div className="flex items-center gap-4 flex-shrink-0">
        {/* Stereo LED Audio Meter */}
        <div className="w-28 hidden md:block">
          <Meter leftLevel={leftMeter} rightLevel={rightMeter} showLabels={false} />
        </div>

        {/* Volume Fader */}
        <div className="flex items-center gap-2 w-32">
          <button
            type="button"
            onClick={() => setIsMuted(!isMuted)}
            className="text-[var(--color-fg-muted)] hover:text-[var(--color-fg)] p-1 rounded-md"
            title={isMuted ? 'Unmute master' : 'Mute master'}
          >
            {isMuted || volume === 0 ? (
              <VolumeX className="w-4 h-4 text-red-400" />
            ) : (
              <Volume2 className="w-4 h-4" />
            )}
          </button>
          <div className="flex-1">
            <Slider
              min={0}
              max={1}
              step={0.01}
              value={isMuted ? 0 : volume}
              onChange={(val) => {
                setVolume(val);
                if (isMuted) setIsMuted(false);
              }}
              formatValue={(v) => `${(v * 100).toFixed(0)}%`}
            />
          </div>
        </div>
      </div>
    </footer>
  );
}
