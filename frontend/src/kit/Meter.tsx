import { memo } from 'react';

export interface MeterProps {
  leftLevel: number;
  rightLevel?: number;
  peakLeft?: number;
  peakRight?: number;
  orientation?: 'horizontal' | 'vertical';
  showLabels?: boolean;
  className?: string;
}

const SEGMENT_COUNT = 20;

function toLinearNormalized(val: number): number {
  if (Number.isNaN(val)) return 0;
  if (val < 0) {
    return Math.max(0, Math.min(1, (val + 60) / 60));
  }
  return Math.max(0, Math.min(1, val));
}

/** Discrete flat meter segment ramp: sky (low) -> amber (mid) -> red (high). Zero green. */
function segmentColorClass(index: number): string {
  const ratio = (index + 1) / SEGMENT_COUNT;
  if (ratio > 0.85) return 'bg-red';
  if (ratio > 0.7) return 'bg-amber';
  return 'bg-sky';
}

function SegmentedBar({
  level,
  peak,
  vertical,
  label,
  showLabel,
}: {
  level: number;
  peak?: number;
  vertical: boolean;
  label: string;
  showLabel: boolean;
}) {
  const activeSegments = Math.round(level * SEGMENT_COUNT);
  const peakIndex = peak !== undefined ? Math.min(SEGMENT_COUNT - 1, Math.floor(peak * SEGMENT_COUNT)) : -1;

  return (
    <div
      className={
        vertical ? 'flex flex-col items-center gap-1' : 'flex items-center gap-2'
      }
    >
      {showLabel && (
        <span className="font-mono text-[10px] font-medium text-muted tabular-nums">
          {label}
        </span>
      )}
      <div
        role="meter"
        aria-label={`${label} audio level`}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.round(level * 100)}
        className={
          vertical
            ? 'flex h-24 w-2 flex-col-reverse gap-0.5 rounded bg-surface-2 p-0.5 border border-border'
            : 'flex h-2.5 flex-1 items-center gap-0.5 rounded bg-surface-2 p-0.5 border border-border'
        }
      >
        {Array.from({ length: SEGMENT_COUNT }, (_, idx) => {
          const isActive = idx < activeSegments || idx === peakIndex;
          return (
            <span
              key={idx}
              className={`rounded-[1px] ${vertical ? 'w-full flex-1' : 'h-full flex-1'} ${
                isActive ? segmentColorClass(idx) : 'bg-track'
              }`}
            />
          );
        })}
      </div>
    </div>
  );
}

export const Meter = memo(function Meter({
  leftLevel,
  rightLevel,
  peakLeft,
  peakRight,
  orientation = 'horizontal',
  showLabels = false,
  className = '',
}: MeterProps) {
  const vertical = orientation === 'vertical';
  const left = toLinearNormalized(leftLevel);
  const right = rightLevel !== undefined ? toLinearNormalized(rightLevel) : undefined;
  const pLeft = peakLeft !== undefined ? toLinearNormalized(peakLeft) : undefined;
  const pRight = peakRight !== undefined ? toLinearNormalized(peakRight) : undefined;

  return (
    <div
      className={`${vertical ? 'inline-flex items-stretch gap-1.5' : 'flex flex-col gap-1'} ${className}`}
    >
      <SegmentedBar
        level={left}
        peak={pLeft}
        vertical={vertical}
        label="L"
        showLabel={showLabels}
      />
      {right !== undefined && (
        <SegmentedBar
          level={right}
          peak={pRight}
          vertical={vertical}
          label="R"
          showLabel={showLabels}
        />
      )}
    </div>
  );
});
