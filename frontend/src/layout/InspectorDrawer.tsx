import { useState } from 'react';
import { X, Sliders, Music, Zap, Check } from 'lucide-react';
import { Button } from '../kit/Button';
import { Slider } from '../kit/Slider';

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
  const [gain, setGain] = useState(0.0); // dB (-60 to +12)
  const [pan, setPan] = useState(0.0); // -1.0 to +1.0
  const [isMuted, setIsMuted] = useState(false);
  const [isSolo, setIsSolo] = useState(false);
  const [isArmed, setIsArmed] = useState(false);
  const [normalized, setNormalized] = useState(false);

  if (!open) return null;

  return (
    <aside
      className="w-[280px] border-l border-[var(--color-border)] bg-[var(--color-surface)] flex flex-col flex-shrink-0 z-20 select-none animate-in slide-in-from-right duration-150"
      aria-label="Property Inspector Drawer"
    >
      {/* Header */}
      <div className="h-10 px-4 border-b border-[var(--color-border)] bg-[var(--color-surface-2)] flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Sliders className="w-4 h-4 text-[var(--color-accent)]" />
          <span className="text-xs font-medium text-[var(--color-fg)]">
            Inspector
          </span>
        </div>
        <Button
          variant="icon"
          size="xs"
          onClick={onClose}
          aria-label="Close Inspector"
          title="Close Inspector (Ctrl+B)"
        >
          <X className="w-4 h-4" />
        </Button>
      </div>

      {/* Drawer Content */}
      <div className="flex-1 overflow-y-auto p-4 space-y-5 text-xs">
        {/* Selected Clip Card */}
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <span className="text-[11px] font-medium text-[var(--color-fg-muted)]">
              Selected Clip
            </span>
            <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-[var(--color-surface-2)] text-[var(--color-accent)]">
              clp_demo_01
            </span>
          </div>

          <div className="p-3 rounded-xl bg-[var(--color-surface-2)] border border-[var(--color-border)] space-y-2.5">
            <div className="flex items-center gap-2">
              <Music className="w-4 h-4 text-[var(--color-accent)]" />
              <div className="font-medium truncate text-[var(--color-fg)]">
                Demo Lead Voice Take
              </div>
            </div>

            <div className="grid grid-cols-2 gap-2 text-[11px] font-mono pt-1">
              <div>
                <span className="text-[var(--color-fg-subtle)]">START: </span>
                <span>0.00s</span>
              </div>
              <div>
                <span className="text-[var(--color-fg-subtle)]">DUR: </span>
                <span>12.50s</span>
              </div>
            </div>

            {/* Normalize Action Button */}
            <Button
              variant={normalized ? 'outline' : 'secondary'}
              size="xs"
              className="w-full mt-1"
              leftIcon={normalized ? <Check className="w-4 h-4 text-[var(--color-fg-muted)]" /> : <Zap className="w-4 h-4 text-[var(--color-fg-muted)]" />}
              onClick={() => setNormalized(!normalized)}
            >
              {normalized ? 'Normalized (-0.2 dBFS)' : 'Normalize Clip'}
            </Button>
          </div>
        </div>

        {/* Track Gain & Pan Controls */}
        <div className="space-y-3 pt-3 border-t border-[var(--color-border)]">
          <div className="flex items-center justify-between">
            <span className="text-[11px] font-medium text-[var(--color-fg-muted)]">
              Track Mixing
            </span>
            <span className="text-[10px] font-mono text-[var(--color-fg-subtle)]">trk_0 (Audio)</span>
          </div>

          <Slider
            label="Gain"
            value={gain}
            min={-36}
            max={12}
            step={0.5}
            unit="dB"
            onChange={setGain}
            formatValue={(v) => `${v > 0 ? `+${v.toFixed(1)}` : v.toFixed(1)} dB`}
          />

          <Slider
            label="Pan"
            value={pan}
            min={-1}
            max={1}
            step={0.05}
            onChange={setPan}
            formatValue={(v) => {
              if (Math.abs(v) < 0.05) return 'Center';
              return v < 0 ? `L ${(Math.abs(v) * 100).toFixed(0)}%` : `R ${(v * 100).toFixed(0)}%`;
            }}
          />

          {/* Mute, Solo, Record Arm Toggles */}
          <div className="grid grid-cols-3 gap-2 pt-1">
            <button
              type="button"
              onClick={() => setIsMuted(!isMuted)}
              className={`h-8 rounded-lg text-xs font-medium transition-[color,background-color,border-color,opacity,transform] border cursor-pointer ${
 isMuted
 ? 'bg-amber-500/20 text-[var(--color-fg-muted)] border-amber-500/40'
 : 'bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] border-[var(--color-border)] hover:bg-[var(--color-surface-3)]'
 }`}
            >
              MUTE
            </button>
            <button
              type="button"
              onClick={() => setIsSolo(!isSolo)}
              className={`h-8 rounded-lg text-xs font-medium transition-[color,background-color,border-color,opacity,transform] border cursor-pointer ${
 isSolo
 ? 'bg-[var(--color-accent)]/20 text-[var(--color-accent)] border-[var(--color-border-strong)]'
 : 'bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] border-[var(--color-border)] hover:bg-[var(--color-surface-3)]'
 }`}
            >
              SOLO
            </button>
            <button
              type="button"
              onClick={() => setIsArmed(!isArmed)}
              className={`h-8 rounded-lg text-xs font-medium transition-[color,background-color,border-color,opacity,transform] border cursor-pointer ${
 isArmed
 ? 'bg-red-500/30 text-red-300 border-red-500/50'
 : 'bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] border-[var(--color-border)] hover:bg-[var(--color-surface-3)]'
 }`}
            >
              ARM
            </button>
          </div>
        </div>

        {/* Audio Engine Specs */}
        <div className="space-y-2 pt-3 border-t border-[var(--color-border)]">
          <div className="text-[11px] font-medium text-[var(--color-fg-muted)]">
            Audio Stream Engine
          </div>
          <div className="p-3 rounded-xl bg-[var(--color-surface-2)] border border-[var(--color-border)] space-y-1.5 text-[11px] font-mono">
            <div className="flex justify-between">
              <span className="text-[var(--color-fg-subtle)]">SAMPLE RATE:</span>
              <span className="text-[var(--color-fg)]">48,000 Hz</span>
            </div>
            <div className="flex justify-between">
              <span className="text-[var(--color-fg-subtle)]">BIT DEPTH:</span>
              <span className="text-[var(--color-fg)]">32-bit Float</span>
            </div>
            <div className="flex justify-between">
              <span className="text-[var(--color-fg-subtle)]">CHANNELS:</span>
              <span className="text-[var(--color-fg)]">2 (Stereo)</span>
            </div>
            <div className="flex justify-between">
              <span className="text-[var(--color-fg-subtle)]">ACTIVE LAB:</span>
              <span className="text-[var(--color-accent)]">{activeWorkspaceName}</span>
            </div>
          </div>
        </div>
      </div>
    </aside>
  );
}
