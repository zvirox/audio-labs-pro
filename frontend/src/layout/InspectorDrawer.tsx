import { useState } from 'react';
import { Sliders, Volume2, X } from 'lucide-react';
import { Button } from '../kit/Button';
import { Slider } from '../kit/Slider';
import { Toggle } from '../kit/Toggle';
import { Meter } from '../kit/Meter';

interface InspectorDrawerProps {
  open: boolean;
  onClose: () => void;
  activeWorkspaceName: string;
}

export function InspectorDrawer({
  open,
  onClose,
  activeWorkspaceName,
}: InspectorDrawerProps) {
  const [gain, setGain] = useState(0.0);
  const [pan, setPan] = useState(0.0);
  const [isMuted, setIsMuted] = useState(false);
  const [isSolo, setIsSolo] = useState(false);
  const [isArmed, setIsArmed] = useState(false);
  const [normalized, setNormalized] = useState(false);

  if (!open) return null;

  return (
    <aside
      aria-label="Context Inspector"
      className="flex w-[280px] shrink-0 flex-col border-l border-border bg-surface text-fg"
    >
      <div className="flex h-11 items-center justify-between border-b border-border px-4">
        <div className="flex items-center gap-2">
          <Sliders className="h-3.5 w-3.5 text-primary-text" />
          <span className="text-[13px] font-semibold">Inspector</span>
        </div>
        <button
          type="button"
          onClick={onClose}
          aria-label="Close inspector"
          className="inline-flex h-6 w-6 items-center justify-center rounded text-muted hover:bg-hover hover:text-fg cursor-pointer"
        >
          <X className="h-3.5 w-3.5" />
        </button>
      </div>

      <div className="flex flex-1 flex-col gap-4 overflow-y-auto p-4">
        <div className="rounded-lg border border-border bg-surface-2 p-3">
          <div className="text-[11px] font-medium uppercase tracking-wider text-faint">
            Active Studio
          </div>
          <div className="mt-1 text-[13px] font-semibold text-fg">
            {activeWorkspaceName}
          </div>
        </div>

        <div className="flex flex-col gap-3 rounded-lg border border-border bg-surface-2 p-3">
          <div className="flex items-center gap-1.5 text-[12px] font-semibold text-fg">
            <Volume2 className="h-3.5 w-3.5 text-sky" />
            <span>Channel Strip</span>
          </div>

          <Slider
            label="Input Gain"
            value={gain}
            min={-60}
            max={12}
            step={0.5}
            unit=" dB"
            formatValue={(v) => `${v > 0 ? '+' : ''}${v.toFixed(1)} dB`}
            onChange={setGain}
          />

          <Slider
            label="Stereo Pan"
            value={pan}
            min={-1}
            max={1}
            step={0.05}
            formatValue={(v) =>
              Math.abs(v) < 0.02
                ? 'Center'
                : v < 0
                ? `${Math.round(Math.abs(v) * 100)}L`
                : `${Math.round(v * 100)}R`
            }
            onChange={setPan}
          />

          <div className="grid grid-cols-3 gap-1.5 pt-1">
            <Button
              size="sm"
              variant={isMuted ? 'secondary' : 'secondary'}
              className={isMuted ? 'border-amber text-amber' : ''}
              onClick={() => setIsMuted((prev) => !prev)}
            >
              Mute
            </Button>
            <Button
              size="sm"
              variant="secondary"
              className={isSolo ? 'border-sky bg-chip-sky text-sky-text' : ''}
              onClick={() => setIsSolo((prev) => !prev)}
            >
              Solo
            </Button>
            <Button
              size="sm"
              variant={isArmed ? 'danger' : 'secondary'}
              onClick={() => setIsArmed((prev) => !prev)}
            >
              Arm
            </Button>
          </div>
        </div>

        <div className="flex flex-col gap-2.5 rounded-lg border border-border bg-surface-2 p-3">
          <div className="text-[12px] font-semibold text-fg">Output Level</div>
          <Meter leftLevel={isMuted ? 0 : 0.62} rightLevel={isMuted ? 0 : 0.58} showLabels />
          <Toggle
            checked={normalized}
            onChange={setNormalized}
            label="Loudness Normalize"
            description="Clamp peaks at -1.0 dBTP"
            size="sm"
          />
        </div>
      </div>
    </aside>
  );
}
