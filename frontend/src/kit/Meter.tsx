import { memo } from 'react';

export interface MeterProps {
  leftLevel: number; // in linear 0.0 - 1.0 or dBFS
  rightLevel?: number;
  peakLeft?: number;
  peakRight?: number;
  orientation?: 'horizontal' | 'vertical';
  showLabels?: boolean;
  className?: string;
}

export const Meter = memo(function Meter({
  leftLevel,
  rightLevel,
  peakLeft = 0,
  peakRight = 0,
  orientation = 'horizontal',
  showLabels = true,
  className = '',
}: MeterProps) {
  // Convert linear 0..1 to percentage or clamp
  const lPercent = Math.max(0, Math.min(100, leftLevel * 100));
  const rPercent = rightLevel !== undefined ? Math.max(0, Math.min(100, rightLevel * 100)) : undefined;

  const isClippedL = peakLeft > 0.98 || leftLevel > 0.98;
  const isClippedR = peakRight > 0.98 || (rightLevel !== undefined && rightLevel > 0.98);

  const isVertical = orientation === 'vertical';

  const renderBar = (percent: number, isClipped: boolean, channel: string) => {
    return (
      <div className={`relative flex-1 bg-[var(--color-surface-3)] rounded-[2px] overflow-hidden border border-[var(--color-border)] ${isVertical ? 'w-2 h-full' : 'h-2 w-full'}`}>
        {/* Gradient fill */}
        <div
          className={`absolute transition-[color,background-color,border-color,opacity,transform] duration-75 ${
 isVertical
 ? 'bottom-0 w-full bg-gradient-to-t from-emerald-500 via-amber-400 to-rose-500'
 : 'left-0 h-full bg-gradient-to-r from-emerald-500 via-amber-400 to-rose-500'
 }`}
          style={isVertical ? { height: `${percent}%` } : { width: `${percent}%` }}
        />
        {/* Clip LED indicator */}
        {isClipped && (
          <div
            className={`absolute ${isVertical ? 'top-0 left-0 right-0 h-1.5' : 'right-0 top-0 bottom-0 w-1.5'} bg-red-500 animate-pulse`}
            title={`${channel} Clipped`}
          />
        )}
      </div>
    );
  };

  return (
    <div className={`flex flex-col gap-1 select-none font-mono text-[9px] ${className}`}>
      {showLabels && (
        <div className="flex justify-between text-[var(--color-fg-subtle)] px-0.5">
          <span>-48</span>
          <span>-24</span>
          <span>-12</span>
          <span>-6</span>
          <span className="text-rose-400">0</span>
        </div>
      )}
      <div className={`flex ${isVertical ? 'flex-row h-24' : 'flex-col'} gap-1`}>
        {renderBar(lPercent, isClippedL, 'L')}
        {rPercent !== undefined && renderBar(rPercent, isClippedR, 'R')}
      </div>
    </div>
  );
});
